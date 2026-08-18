#!/usr/bin/env bash
# Session-id dispatcher for the benchmark agent runtime.
#
# Delegates session-id extraction to the active agent driver's
# agent_session_id. The codex driver keeps the JSONL/thread_id extraction
# verbatim; other agents implement their own stream parsing and degrade to an
# empty session id when the stream has no matching key.
#
# The active driver is resolved via REPO_ROOT (exported by benchmark-env.sh for
# generated case scripts) with a dirname fallback for standalone invocation
# from the repo checkout.

set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <worker.jsonl>" >&2
    exit 64
fi

# Deliberately not shared with telemetry.sh's near-identical copy: that one must
# degrade to the documented telemetry keys, this one may fail closed. An
# unresolvable driver exits 64 here, and the harness taints the case.
source_agent_lib() {
    if [ -n "${REPO_ROOT:-}" ] && [ -f "$REPO_ROOT/benchmark/planning/runtime/lib-agent.sh" ]; then
        source "$REPO_ROOT/benchmark/planning/runtime/lib-agent.sh"
    elif [ -f "$(cd "$(dirname "$0")" && pwd)/runtime/lib-agent.sh" ]; then
        source "$(cd "$(dirname "$0")/runtime" && pwd)/lib-agent.sh"
    else
        echo "benchmark runtime not found (set REPO_ROOT or run from the repo)" >&2
        exit 64
    fi
}
source_agent_lib

agent_session_id "$1"