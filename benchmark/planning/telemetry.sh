#!/usr/bin/env bash
# Telemetry dispatcher for the benchmark agent runtime.
#
# Delegates token accounting to the active agent driver's agent_telemetry.
# For codex this is the SQLite/rollout parser; for agents without a documented
# store it degrades honestly to unavailable. Never fabricates token numbers.
#
# The active driver is resolved via REPO_ROOT (exported by benchmark-env.sh for
# generated case scripts) with a dirname fallback for standalone invocation
# from the repo checkout.

set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <session-id>" >&2
    exit 64
fi

THREAD_ID="$1"
if [[ ! "$THREAD_ID" =~ ^[A-Za-z0-9_-]+$ ]]; then
    printf 'thread_id=%s\n' "$THREAD_ID"
    printf 'usage_records=0\n'
    printf 'total_usage_tokens=0\n'
    printf 'telemetry_status=unavailable:invalid thread id\n'
    exit 0
fi

source_agent_lib() {
    if [ -n "${REPO_ROOT:-}" ] && [ -f "$REPO_ROOT/benchmark/planning/runtime/lib-agent.sh" ]; then
        source "$REPO_ROOT/benchmark/planning/runtime/lib-agent.sh"
    elif [ -f "$(cd "$(dirname "$0")" && pwd)/runtime/lib-agent.sh" ]; then
        source "$(cd "$(dirname "$0")/runtime" && pwd)/lib-agent.sh"
    else
        printf 'thread_id=%s\n' "$THREAD_ID"
        printf 'usage_records=unavailable\n'
        printf 'total_usage_tokens=unavailable\n'
        printf 'telemetry_status=unavailable:benchmark runtime not found\n'
        exit 0
    fi
}
source_agent_lib

agent_telemetry "$THREAD_ID"