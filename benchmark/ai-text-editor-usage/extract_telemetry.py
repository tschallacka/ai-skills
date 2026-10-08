"""Extracts token/tool-call/timing analytics from one real Claude Code
session transcript (~/.claude/projects/<escaped-cwd>/<session-id>.jsonl).

Same source of truth benchmark/planning/runtime/claude/agent.sh's own
agent_telemetry reads (assistant.message.usage), extended with per-tool call
counts, wall-clock duration, a main-chain/subagent (isSidechain) token and
tool-call split, and whether/when the worker reached for ai-text-editor on
its own -- the prompt never names the tool, so this is the only record of
whether its own description sold it, and how many stock-tool calls came
first.

Usage:
  extract_telemetry.py <transcript>              key=value lines (telemetry.sh's own contract)
  extract_telemetry.py <transcript> --json        full structured report on stdout
"""

import json
import re
import sys
from collections import Counter

MENTION_PATTERN = re.compile(r"ai[-_]text[-_]editor", re.IGNORECASE)
AI_TEXT_EDITOR_TOOL_PREFIX = "mcp__ai-text-editor__"


def load(transcript_path):
    records = []
    with open(transcript_path, encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            try:
                records.append(json.loads(line))
            except json.JSONDecodeError:
                continue
    return records


def _usage_tokens(usage):
    input_tokens = usage.get("input_tokens") or 0
    cache_creation_tokens = usage.get("cache_creation_input_tokens") or 0
    cache_read_tokens = usage.get("cache_read_input_tokens") or 0
    output_tokens = usage.get("output_tokens") or 0
    thinking_tokens = (usage.get("output_tokens_details") or {}).get("thinking_tokens") or 0
    total = input_tokens + cache_creation_tokens + cache_read_tokens + output_tokens
    return {
        "input_tokens": input_tokens,
        "cache_creation_input_tokens": cache_creation_tokens,
        "cache_read_input_tokens": cache_read_tokens,
        "output_tokens": output_tokens,
        "thinking_tokens": thinking_tokens,
        "total_usage_tokens": total,
    }


def _add(totals, part):
    for key, value in part.items():
        totals[key] = totals.get(key, 0) + value


def _mention_snippets(text, limit=200, max_snippets=3):
    snippets = []
    for match in MENTION_PATTERN.finditer(text):
        start = max(match.start() - limit // 2, 0)
        end = min(match.end() + limit // 2, len(text))
        snippets.append(text[start:end].strip())
        if len(snippets) >= max_snippets:
            break
    return snippets


def analyze(records):
    usage_records = 0
    models = set()
    timestamps = []

    main_totals = {}
    sidechain_totals = {}
    main_tool_calls = Counter()
    sidechain_tool_calls = Counter()

    tool_call_sequence = []  # every tool_use block, in file order
    mention_snippets = []
    subagent_dispatch_count = 0  # "Task" tool_use blocks in the main chain

    for event in records:
        ts = event.get("timestamp")
        if ts:
            timestamps.append(ts)

        message = event.get("message")
        if not isinstance(message, dict):
            continue

        is_sidechain = bool(event.get("isSidechain"))

        if event.get("type") == "assistant":
            usage = message.get("usage")
            if isinstance(usage, dict):
                usage_records += 1
                part = _usage_tokens(usage)
                _add(sidechain_totals if is_sidechain else main_totals, part)
            model = message.get("model")
            if model:
                models.add(model)

        content = message.get("content")
        if isinstance(content, list):
            for block in content:
                if not isinstance(block, dict):
                    continue
                block_type = block.get("type")
                if block_type == "tool_use":
                    name = block.get("name", "unknown")
                    (sidechain_tool_calls if is_sidechain else main_tool_calls)[name] += 1
                    tool_call_sequence.append({"name": name, "is_sidechain": is_sidechain})
                    if name == "Task" and not is_sidechain:
                        subagent_dispatch_count += 1
                elif block_type in ("text", "thinking"):
                    text = block.get("text") or block.get("thinking") or ""
                    if text and MENTION_PATTERN.search(text):
                        mention_snippets.extend(_mention_snippets(text))

    combined_totals = dict(main_totals)
    _add(combined_totals, sidechain_totals)
    for key in (
        "input_tokens",
        "cache_creation_input_tokens",
        "cache_read_input_tokens",
        "output_tokens",
        "thinking_tokens",
        "total_usage_tokens",
    ):
        combined_totals.setdefault(key, 0)
        main_totals.setdefault(key, 0)
        sidechain_totals.setdefault(key, 0)

    combined_tool_calls = main_tool_calls + sidechain_tool_calls

    first_adoption_index = None
    calls_before_adoption = Counter()
    for idx, call in enumerate(tool_call_sequence):
        if call["name"].startswith(AI_TEXT_EDITOR_TOOL_PREFIX):
            first_adoption_index = idx
            break
        calls_before_adoption[call["name"]] += 1

    ai_text_editor_calls = sum(
        count for name, count in combined_tool_calls.items() if name.startswith(AI_TEXT_EDITOR_TOOL_PREFIX)
    )

    duration_seconds = None
    if len(timestamps) >= 2:
        timestamps.sort()
        from datetime import datetime

        def parse(ts):
            return datetime.fromisoformat(ts.replace("Z", "+00:00"))

        try:
            duration_seconds = (parse(timestamps[-1]) - parse(timestamps[0])).total_seconds()
        except ValueError:
            duration_seconds = None

    return {
        "usage_records": usage_records,
        **combined_totals,
        "tool_calls_total": sum(combined_tool_calls.values()),
        "tool_calls_by_name": dict(combined_tool_calls),
        "main_chain": {
            **main_totals,
            "tool_calls_total": sum(main_tool_calls.values()),
            "tool_calls_by_name": dict(main_tool_calls),
        },
        "subagent_sidechain": {
            **sidechain_totals,
            "tool_calls_total": sum(sidechain_tool_calls.values()),
            "tool_calls_by_name": dict(sidechain_tool_calls),
        },
        "subagent_dispatch_count": subagent_dispatch_count,
        "ai_text_editor_adoption": {
            "adopted": first_adoption_index is not None,
            "total_calls": ai_text_editor_calls,
            "first_call_sequence_index": first_adoption_index,
            "stock_tool_calls_before_adoption": dict(calls_before_adoption),
        },
        "ai_text_editor_text_mentions": {
            "count": len(mention_snippets),
            "snippets": mention_snippets,
        },
        "models": sorted(models),
        "duration_seconds": duration_seconds,
        "record_count": len(records),
    }


def main():
    if len(sys.argv) < 2:
        print("Usage: extract_telemetry.py <transcript> [--json]", file=sys.stderr)
        return 64

    transcript_path = sys.argv[1]
    as_json = "--json" in sys.argv[2:]

    try:
        records = load(transcript_path)
    except OSError as error:
        print(f"telemetry_status=unavailable:{error}", file=sys.stderr)
        return 2

    report = analyze(records)

    if report["usage_records"] == 0:
        return 1

    if as_json:
        print(json.dumps(report, indent=2))
        return 0

    print(f"telemetry_db={transcript_path}")
    print(f"usage_records={report['usage_records']}")
    print(f"total_usage_tokens={report['total_usage_tokens']}")
    print("telemetry_source=claude-transcript-assistant-usage")
    print("telemetry_status=available")
    return 0


if __name__ == "__main__":
    sys.exit(main())
