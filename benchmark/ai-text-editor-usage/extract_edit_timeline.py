"""Builds a chronological edit-mechanism timeline for one ai-text-editor-usage
run: every file-mutating tool call -- stock Edit/Write/MultiEdit versus
mcp__ai-text-editor__* mutating verbs -- in order, with its wall-clock
timestamp, cross-referenced against when the two external (on-disk) probes
actually landed. Answers three things extract_telemetry.py's single
"adopted: true/false" bit does not: how many edits each mechanism actually
made, exactly when (if ever) the worker switched mechanism mid-task, and
whether that switch followed a real on-disk change closely enough to look
causal rather than coincidental.

Usage:
  extract_edit_timeline.py <transcript> <external-edit.log> <external-format.log>

Always prints the full JSON report to stdout.
"""

import json
import re
import sys
from datetime import datetime

# Verbs that actually mutate the document or the file on disk. `resolve` is
# split out on its own rather than lumped into "edit": it is specifically
# the server's external-change-detected decision point, which is the one
# action this file exists to correlate against the external probes'
# own landing time. Everything else ai-text-editor exposes (open, read,
# search, history, index, cursor, move, capabilities, resources,
# jump_points, page, copy, close, job_*) is navigation/metadata, not an edit.
MCP_MUTATING_VERBS = {"insert", "replace", "large_edit", "restore", "undo", "redo", "save", "save_as"}
MCP_RESOLVE_VERBS = {"resolve"}
MCP_PREFIX = "mcp__ai-text-editor__"

INTERNAL_MUTATING_TOOLS = {"Edit", "Write", "MultiEdit", "NotebookEdit"}

EXTERNAL_LOG_TS = re.compile(r"at (\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z)")


def parse_ts(ts):
    return datetime.fromisoformat(ts.replace("Z", "+00:00"))


def classify(name):
    """-> (mechanism, action) for one tool_use name."""
    if name.startswith(MCP_PREFIX):
        verb = name[len(MCP_PREFIX):]
        if verb in MCP_MUTATING_VERBS:
            return "mcp", "edit"
        if verb in MCP_RESOLVE_VERBS:
            return "mcp", "resolve"
        return "mcp", "read"
    if name in INTERNAL_MUTATING_TOOLS:
        return "internal", "edit"
    if name in ("Read", "Glob", "Grep"):
        return "internal", "read"
    return "other", "other"


def load_transcript(path):
    events = []
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue
            ts = record.get("timestamp")
            message = record.get("message")
            if not ts or not isinstance(message, dict):
                continue
            content = message.get("content")
            if not isinstance(content, list):
                continue
            for block in content:
                if isinstance(block, dict) and block.get("type") == "tool_use":
                    events.append({"timestamp": ts, "name": block.get("name", "unknown")})
    return events


def load_external_landings(log_path, landing_marker):
    """Returns every timestamp on a line containing landing_marker, in file order."""
    landings = []
    try:
        with open(log_path, encoding="utf-8") as handle:
            text = handle.read()
    except OSError:
        return landings
    for line in text.splitlines():
        if landing_marker not in line:
            continue
        match = EXTERNAL_LOG_TS.search(line)
        if match:
            landings.append(match.group(1))
    return landings


def nearest_preceding(landings, at_ts):
    """Among timestamps <= at_ts, the latest one, with its age in seconds."""
    best = None
    for landed in landings:
        if landed <= at_ts:
            if best is None or landed > best:
                best = landed
    if best is None:
        return None, None
    age = (parse_ts(at_ts) - parse_ts(best)).total_seconds()
    return best, age


def build(transcript_path, edit_log_path, format_log_path):
    raw_events = load_transcript(transcript_path)
    if not raw_events:
        return {"error": "no tool_use events found in transcript"}

    task_start = raw_events[0]["timestamp"]

    timeline = []
    for idx, event in enumerate(raw_events):
        mechanism, action = classify(event["name"])
        timeline.append(
            {
                "idx": idx,
                "timestamp": event["timestamp"],
                "offset_seconds": (parse_ts(event["timestamp"]) - parse_ts(task_start)).total_seconds(),
                "name": event["name"],
                "mechanism": mechanism,
                "action": action,
            }
        )

    edit_calls = [e for e in timeline if e["action"] in ("edit", "resolve")]

    internal_edit_count = sum(1 for e in edit_calls if e["mechanism"] == "internal" and e["action"] == "edit")
    mcp_edit_count = sum(1 for e in edit_calls if e["mechanism"] == "mcp" and e["action"] == "edit")
    mcp_resolve_count = sum(1 for e in edit_calls if e["action"] == "resolve")

    switches = []
    previous_mechanism = None
    for e in edit_calls:
        if e["action"] != "edit":
            continue
        if previous_mechanism is not None and e["mechanism"] != previous_mechanism:
            switches.append(
                {
                    "at_idx": e["idx"],
                    "timestamp": e["timestamp"],
                    "offset_seconds": e["offset_seconds"],
                    "from": previous_mechanism,
                    "to": e["mechanism"],
                }
            )
        previous_mechanism = e["mechanism"]

    tax_rate_landings = load_external_landings(edit_log_path, "TAX_RATE changed")
    reindent_landings = load_external_landings(format_log_path, "external-format: done")
    all_landings = sorted(tax_rate_landings + reindent_landings)

    def correlate(event):
        landed_at, age = nearest_preceding(all_landings, event["timestamp"])
        return {
            **event,
            "nearest_preceding_disk_change": landed_at,
            "seconds_after_disk_change": age,
            "plausibly_disk_triggered": age is not None and 0 <= age <= 15,
        }

    switch_to_mcp_events = [correlate(s) for s in switches if s["to"] == "mcp"]
    resolve_events = [correlate(e) for e in edit_calls if e["action"] == "resolve"]

    return {
        "task_start": task_start,
        "internal_edit_count": internal_edit_count,
        "mcp_edit_count": mcp_edit_count,
        "mcp_resolve_count": mcp_resolve_count,
        "mechanism_switch_count": len(switches),
        "disk_landings": {"tax_rate_bump": tax_rate_landings, "reindent": reindent_landings},
        "switches": switches,
        "switch_to_mcp_disk_correlation": switch_to_mcp_events,
        "resolve_disk_correlation": resolve_events,
        "timeline": timeline,
    }


def main():
    if len(sys.argv) < 4:
        print(
            "Usage: extract_edit_timeline.py <transcript> <external-edit.log> <external-format.log> [--json]",
            file=sys.stderr,
        )
        return 64

    transcript_path, edit_log_path, format_log_path = sys.argv[1], sys.argv[2], sys.argv[3]
    report = build(transcript_path, edit_log_path, format_log_path)
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
