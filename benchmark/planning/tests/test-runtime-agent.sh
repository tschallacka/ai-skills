#!/usr/bin/env bash
# Runtime contract tests for the benchmark agent runtime.
#
# Asserts each runtime/<agent>/agent.sh exports the reserved driver contract
# (including agent_model_env/agent_default_model), active-agent resolution
# (BENCHMARK_AGENT default codex, unknown fails closed *through lib-agent.sh*,
# which is the path every harness entry point takes), that each argv builder
# reports the workspace as AGENT_CWD, and scaffolder idempotence / no-clobber
# against an isolated temp runtime replica (never the live repo).

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
runtime="$root/runtime"

reserved_functions=(
    agent_argv_worker
    agent_argv_reviewer
    agent_argv_analyzer
    agent_session_id
    agent_telemetry
)
reserved_variables=(
    AGENT_NAME
    agent_bin
    agent_model_env
    agent_default_model
)

for agent in codex opencode claude; do
    driver="$runtime/$agent/agent.sh"
    [ -f "$driver" ] || { echo "missing driver: $driver" >&2; exit 1; }
    bash -n "$driver" || { echo "syntax error: $driver" >&2; exit 1; }

    # Shell inherits variables/functions from the parent; capture a clean
    # marker so sourcing the driver is what defines the contract symbols.
    # PORTABILITY(date-nanoseconds)
    marker="__rt_probe_$(date -u +%s)_${RANDOM}${RANDOM}__"
    out="$(bash -c '
        set -euo pipefail
        driver="$1"
        marker="$2"
        source "$driver"
        for fn in '"${reserved_functions[*]}"'; do
            if ! declare -F "$fn" >/dev/null 2>&1; then
                echo "MISSING_FUNCTION:$fn"
            fi
        done
        for var in '"${reserved_variables[*]}"'; do
            if [ -z "${!var:-}" ]; then
                echo "MISSING_VARIABLE:$var"
            fi
        done
        echo "OK:$AGENT_NAME"
    ' _ "$driver" "$marker")"

    if [ -n "$(printf '%s\n' "$out" | grep -E 'MISSING_' || true)" ]; then
        printf 'contract missing for %s:\n%s\n' "$agent" "$out" >&2
        exit 1
    fi
    echo "runtime contract: $agent ($out)"
done

# Active-agent resolution: unset -> codex default.
export -n BENCHMARK_AGENT 2>/dev/null || true
if [ -n "${BENCHMARK_AGENT:-}" ]; then
    unset BENCHMARK_AGENT
fi
resolve_out="$(bash -c '
    set -euo pipefail
    source "$1/agent-env.sh"
    resolve_active_agent "$1"
    echo "$AGENT_DRIVER"
' _ "$runtime")"
[ "$resolve_out" = "codex" ] || { echo "default resolver != codex: $resolve_out" >&2; exit 1; }

# Explicit override selects an installed agent.
for agent in codex opencode claude; do
    out="$(BENCHMARK_AGENT="$agent" bash -c '
        set -euo pipefail
        source "$1/agent-env.sh"
        resolve_active_agent "$1"
        echo "$AGENT_DRIVER"
    ' _ "$runtime")"
    [ "$out" = "$agent" ] || { echo "BENCHMARK_AGENT=$agent resolved to $out" >&2; exit 1; }
done

# Unknown agent fails closed (exit 64).
if BENCHMARK_AGENT=no-such-agent bash -c '
    set -euo pipefail
    source "$1/agent-env.sh"
    resolve_active_agent "$1"
' _ "$runtime" >/dev/null 2>&1; then
    echo "unknown BENCHMARK_AGENT did not fail closed" >&2
    exit 1
fi

# Unknown agent must fail closed through lib-agent.sh too: `if ! cmd; then exit
# $?` yields the negated status 0, which turned a typo'd BENCHMARK_AGENT into an
# empty batch that CI read as a pass.
lib_out="$(BENCHMARK_AGENT=no-such-agent bash -c '
    source "$1/lib-agent.sh"
    echo "REACHED_AFTER_SOURCE"
' _ "$runtime" 2>/dev/null || true)"
lib_code=0
BENCHMARK_AGENT=no-such-agent bash -c '
    source "$1/lib-agent.sh"
' _ "$runtime" >/dev/null 2>&1 || lib_code=$?
[ "$lib_code" -eq 64 ] || { echo "lib-agent.sh swallowed the fail-closed status: exit $lib_code (want 64)" >&2; exit 1; }
case "$lib_out" in
    *REACHED_AFTER_SOURCE*) echo "lib-agent.sh continued after an unresolvable agent" >&2; exit 1 ;;
esac
echo "runtime fail-closed: lib-agent.sh propagates exit 64 for an unknown BENCHMARK_AGENT"

# lib-agent.sh bootstraps the selected driver and the launcher.
for agent in codex opencode claude; do
    tmp="$(mktemp -d "${TMPDIR:-/tmp}/runtime-argv.$agent.XXXXXX")"
    trap 'rm -rf -- "$tmp"' EXIT
    mkdir -p "$tmp/ws" "$tmp/cap"
    printf 'prompt' > "$tmp/prompt.md"
    printf 'spec\n' > "$tmp/cap/task-spec.md"
    out="$(BENCHMARK_AGENT="$agent" bash -c '
        set -euo pipefail
        source "$1/lib-agent.sh"
        agent_argv_worker "$2/ws" "$2/cap" "$2/prompt.md"
        printf "%s\\n" "${AGENT_ARGV[0]}"
        printf "%s\\n" "${AGENT_CWD:-}"
    ' _ "$runtime" "$tmp")"
    argv0="$(printf '%s\n' "$out" | sed -n '1p')"
    argv_cwd="$(printf '%s\n' "$out" | sed -n '2p')"
    [ -n "$argv0" ] || { echo "$agent: empty AGENT_ARGV" >&2; exit 1; }
    # Every driver must report the workspace as the agent's cwd: codex passes
    # -C, opencode --dir, and claude has no cwd flag at all, so launch_agent
    # cds to AGENT_CWD for all three.
    [ "$argv_cwd" = "$tmp/ws" ] || { echo "$agent: AGENT_CWD is '$argv_cwd', want $tmp/ws" >&2; exit 1; }
    echo "runtime argv0: $agent -> $argv0 (cwd=$argv_cwd)"
    trap - EXIT
    rm -rf -- "$tmp"
done

# Scaffolder: first scaffold creates a working driver; second refuses (no
# clobber); runs against a temp replica of the runtime, never the live repo.
tmp="$(mktemp -d "${TMPDIR:-/tmp}/runtime-scaffold.XXXXXX")"
trap 'rm -rf -- "$tmp"' EXIT
cp -R "$runtime/." "$tmp/runtime"
scaffold="$tmp/runtime/scaffold-agent.sh"
bash "$scaffold" probe probe PROBE_MODEL probe-9 >/dev/null
[ -f "$tmp/runtime/probe/agent.sh" ] || { echo "first scaffold did not create driver" >&2; exit 1; }
if bash "$scaffold" probe probe PROBE_MODEL probe-9 >/dev/null 2>&1; then
    echo "second scaffold did not refuse to clobber" >&2
    exit 1
fi
grep -Fq 'agent_model_env="PROBE_MODEL"' "$tmp/runtime/probe/agent.sh"
grep -Fq 'agent_default_model="probe-9"' "$tmp/runtime/probe/agent.sh"
# The scaffolder's own default output must satisfy the contract test above: an
# empty agent_default_model is a MISSING_VARIABLE.
bash "$scaffold" probedefault >/dev/null
default_model="$(sed -nE 's/^agent_default_model="(.*)"$/\1/p' "$tmp/runtime/probedefault/agent.sh")"
[ -n "$default_model" ] || { echo "scaffolder default left agent_default_model empty" >&2; exit 1; }
# A scaffolded-but-unimplemented driver must *return*, not exit: drivers are
# sourced, so `exit 65` in a stub terminated the whole harness instead of
# failing one case with a status the caller can trap.
stub_out="$(bash -c '
    source "$1"
    rc=0
    agent_argv_worker /tmp /tmp /dev/null || rc=$?
    printf "STUB_RC=%s\n" "$rc"
' _ "$tmp/runtime/probedefault/agent.sh" 2>/dev/null || true)"
tmpl_code=0
[ "$stub_out" = "STUB_RC=65" ] || tmpl_code=1
[ "$tmpl_code" -eq 0 ] || { echo "template stub exits instead of returning 65 (got '$stub_out')" >&2; exit 1; }
echo "runtime template: unimplemented stub returns 65 without exiting; scaffolder default model '$default_model'"
echo "runtime scaffold: first-scaffold created driver, second-scaffold refused (idempotent no-clobber)"

printf 'Runtime contract tests passed.\n'