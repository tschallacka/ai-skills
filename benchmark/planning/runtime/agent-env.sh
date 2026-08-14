#!/usr/bin/env bash
# Active-agent resolver for the benchmark runtime.
#
# Resolves which agent driver (runtime/<agent>/agent.sh) is active and exports
# AGENT_DRIVER. Precedence:
#   1. BENCHMARK_AGENT environment override (must have a driver; else fail closed).
#   2. Unset -> codex (the unchanged default harness path needs no env).
#   3. codex driver missing -> best-effort detection of exactly one installed
#      CLI among the available drivers; otherwise fail closed.
#
# Sourcing this file does not select anything; call resolve_active_agent with
# the runtime directory.

set -euo pipefail

resolve_active_agent() {
    local runtime_dir="$1"
    if [ ! -d "$runtime_dir" ]; then
        echo "agent-env: runtime directory not found: $runtime_dir" >&2
        return 64
    fi

    if [ -n "${BENCHMARK_AGENT:-}" ]; then
        if [ -f "$runtime_dir/$BENCHMARK_AGENT/agent.sh" ]; then
            AGENT_DRIVER="$BENCHMARK_AGENT"
            export AGENT_DRIVER
            return 0
        fi
        echo "agent-env: BENCHMARK_AGENT set to unknown agent '$BENCHMARK_AGENT'; no driver at $runtime_dir/$BENCHMARK_AGENT/agent.sh" >&2
        return 64
    fi

    if [ -f "$runtime_dir/codex/agent.sh" ]; then
        AGENT_DRIVER="codex"
        export AGENT_DRIVER
        return 0
    fi

    local -a present=()
    local candidate name
    for candidate in "$runtime_dir"/*/; do
        [ -d "$candidate" ] || continue
        name="$(basename "$candidate")"
        [ "$name" = "TEMPLATE" ] && continue
        if [ -f "$candidate/agent.sh" ] && command -v "$name" >/dev/null 2>&1; then
            present+=("$name")
        fi
    done
    if [ "${#present[@]}" -eq 1 ]; then
        AGENT_DRIVER="${present[0]}"
        export AGENT_DRIVER
        return 0
    fi
    echo "agent-env: no active agent driver resolved (BENCHMARK_AGENT unset, codex driver missing, ${#present[@]} installed CLI candidates detected)" >&2
    return 64
}