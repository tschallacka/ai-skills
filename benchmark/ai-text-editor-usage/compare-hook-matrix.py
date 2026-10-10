"""Builds a side-by-side report across the eight lever combinations
run-hook-matrix-sweep.sh produces under one sweep directory (each a
treatment-only case named after its "-"-joined levers: "none", "mcp",
"edit", "read", "mcp-edit", "mcp-read", "edit-read", "mcp-edit-read").

Usage: compare-hook-matrix.py <sweep-dir>
Writes <sweep-dir>/hook-matrix-comparison.md and prints it to stdout.
"""

import json
import sys
from pathlib import Path

ORDER = ["none", "mcp", "edit", "read", "mcp-edit", "mcp-read", "edit-read", "mcp-edit-read"]


def load_json(path):
    try:
        with open(path, encoding="utf-8") as handle:
            return json.load(handle)
    except (OSError, json.JSONDecodeError):
        return {}


def load_text(path):
    try:
        with open(path, encoding="utf-8") as handle:
            return handle.read()
    except OSError:
        return ""


def case_report(case_dir):
    telemetry = load_json(case_dir / "telemetry.json")
    score = load_json(case_dir / "score.json")
    timeline = load_json(case_dir / "edit-timeline.json")

    wall_seconds = None
    levers = None
    eval_text = load_text(case_dir / "evaluation.md")
    for line in eval_text.splitlines():
        if line.startswith("- Wall clock:"):
            wall_seconds = line.split(":", 1)[1].strip()
        if line.startswith("- Levers on:"):
            levers = line.split(":", 1)[1].strip()

    adoption = telemetry.get("ai_text_editor_adoption", {})
    mentions = telemetry.get("ai_text_editor_text_mentions", {})

    return {
        "levers": levers,
        "wall_seconds": wall_seconds,
        "adopted": adoption.get("adopted", "n/a"),
        "ai_text_editor_calls": adoption.get("total_calls", "n/a"),
        "first_call_index": adoption.get("first_call_sequence_index", "n/a"),
        "text_mentions": mentions.get("count", "n/a"),
        "mention_snippets": mentions.get("snippets", []),
        "internal_edit_count": timeline.get("internal_edit_count", "n/a"),
        "mcp_edit_count": timeline.get("mcp_edit_count", "n/a"),
        "mcp_resolve_count": timeline.get("mcp_resolve_count", "n/a"),
        "mechanism_switch_count": timeline.get("mechanism_switch_count", "n/a"),
        "switch_to_mcp_disk_correlation": timeline.get("switch_to_mcp_disk_correlation", []),
        "resolve_disk_correlation": timeline.get("resolve_disk_correlation", []),
        "tax_rate_preserved": score.get("tax_rate_external_edit_preserved", "n/a"),
        "reformat_preserved": score.get("external_reformat_preserved", "n/a"),
        "valid_python": score.get("valid_python", "n/a"),
        "outcome": score.get("outcome", "n/a"),
        "asked_for_feedback": score.get("asked_for_feedback", "n/a"),
        "feedback_phrase_matched": score.get("feedback_phrase_matched"),
        "worker_final_summary": score.get("worker_final_summary"),
    }


def main():
    if len(sys.argv) != 2:
        print("Usage: compare-hook-matrix.py <sweep-dir>", file=sys.stderr)
        return 64

    sweep_dir = Path(sys.argv[1])
    available = {
        p.name: p
        for p in sweep_dir.iterdir()
        if p.is_dir() and (p / "treatment").is_dir()
    }
    case_names = [name for name in ORDER if name in available] + sorted(
        name for name in available if name not in ORDER
    )
    if not case_names:
        print(f"compare-hook-matrix.py: no hook-condition case dirs found under {sweep_dir}", file=sys.stderr)
        return 1

    lines = [
        f"# ai-text-editor-usage lever factorial: {sweep_dir.name}",
        "",
        "One treatment-condition run per combination of the three levers the",
        "TOOL AUTHOR controls -- the MCP server's own top-level `instructions`",
        "pitch (`mcp`), and editor-gate-plugin's Edit/Write (`edit`) and Read",
        "(`read`) PreToolUse hooks -- the full 2^3 factorial, each lever",
        "independently on or off. `task-prompt.md` is byte-identical across",
        "every row: nothing an end user would have to type ever changes.",
        "",
        "| levers | adopted | internal edits | mcp edits | resolves | switches | outcome | asked for feedback | wall |",
        "|---|---|---|---|---|---|---|---|---|",
    ]

    reports = {}
    for name in case_names:
        report = case_report(available[name] / "treatment")
        reports[name] = report
        lines.append(
            "| {name} | {adopted} | {internal} | {mcp} | {resolves} | {switches} | {outcome} | {asked} | {wall} |".format(
                name=name,
                adopted=report["adopted"],
                internal=report["internal_edit_count"],
                mcp=report["mcp_edit_count"],
                resolves=report["mcp_resolve_count"],
                switches=report["mechanism_switch_count"],
                outcome=report["outcome"],
                asked=report["asked_for_feedback"],
                wall=report["wall_seconds"],
            )
        )

    lines += ["", "## Disk-change correlation, per condition", ""]
    for name in case_names:
        report = reports[name]
        lines.append(f"### {name}")
        any_correlation = False
        for s in report["switch_to_mcp_disk_correlation"]:
            any_correlation = True
            lines.append(
                f"- switch internal->mcp at +{s['offset_seconds']}s, "
                f"{s['seconds_after_disk_change']}s after the nearest disk change "
                f"(plausibly disk-triggered: {s['plausibly_disk_triggered']})"
            )
        for r in report["resolve_disk_correlation"]:
            any_correlation = True
            lines.append(
                f"- resolve at +{r['offset_seconds']}s, "
                f"{r['seconds_after_disk_change']}s after the nearest disk change "
                f"(plausibly disk-triggered: {r['plausibly_disk_triggered']})"
            )
        if not any_correlation:
            lines.append("(no switch-to-mcp or resolve events in this run)")
        lines.append("")

    lines += ["## Worker's own final summary, per condition", ""]
    for name in case_names:
        report = reports[name]
        lines.append(f"### {name}")
        if report["asked_for_feedback"]:
            lines.append(f"**Asked for feedback** -- matched phrase: `{report['feedback_phrase_matched']}`")
        lines.append("```")
        lines.append(report["worker_final_summary"] or "(no result text)")
        lines.append("```")
        lines.append("")

    lines += ["## Mentions of ai-text-editor in reasoning, per condition", ""]
    for name in case_names:
        report = reports[name]
        lines.append(f"### {name}")
        lines.append("```")
        lines.append("\n---\n".join(report["mention_snippets"]) or "(none)")
        lines.append("```")
        lines.append("")

    report_text = "\n".join(lines) + "\n"
    (sweep_dir / "hook-matrix-comparison.md").write_text(report_text, encoding="utf-8")
    print(report_text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
