#!/usr/bin/env bash
# MODE: DEV
# Token telemetry for one `claude -p` run, by session id. Adapted from
# benchmark/planning/runtime/claude/agent.sh's own agent_telemetry: sums
# input_tokens/cache_creation_input_tokens/cache_read_input_tokens/
# output_tokens across every assistant message in the matching on-disk
# session transcript. Degrades honestly to unavailable rather than guessing.
#
# Usage: telemetry.sh <session-id>
# Prints: thread_id=..., usage_records=..., total_usage_tokens=...,
#         telemetry_source=..., telemetry_status=...

set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <session-id>" >&2
    exit 64
fi

SESSION_ID="$1"
TRANSCRIPT="$(find "${CLAUDE_PROJECTS_DIR:-$HOME/.claude/projects}" -type f -name "${SESSION_ID}.jsonl" -print -quit 2>/dev/null || true)"

printf 'thread_id=%s\n' "$SESSION_ID"

if [ -z "$TRANSCRIPT" ]; then
    printf 'usage_records=0\n'
    printf 'total_usage_tokens=0\n'
    printf 'telemetry_status=unavailable:no claude session transcript found\n'
    exit 0
fi

if ! command -v python3 >/dev/null 2>&1; then
    printf 'usage_records=unavailable\n'
    printf 'total_usage_tokens=unavailable\n'
    printf 'telemetry_status=unavailable:python3 required to read claude transcript\n'
    exit 0
fi

if python3 "$(dirname "$0")/extract_telemetry.py" "$TRANSCRIPT"; then
    exit 0
fi

printf 'usage_records=0\n'
printf 'total_usage_tokens=0\n'
printf 'telemetry_status=unavailable:no usable claude usage in session transcript\n'
