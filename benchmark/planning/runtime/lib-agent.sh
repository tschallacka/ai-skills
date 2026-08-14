#!/usr/bin/env bash
# Shared launcher and argv machinery for the benchmark runtime.
#
# Single source of truth for setsid/timeout/process-group capture and argv
# emission. Sourcing this file:
#   * computes RUNTIME_DIR from its own location (runtime/),
#   * sources agent-env.sh and resolves the active agent (AGENT_DRIVER),
#   * sources runtime/$AGENT_DRIVER/agent.sh (the reserved-contract driver).
#
# After sourcing, the driver's agent_argv_worker/agent_argv_reviewer/
# agent_argv_analyzer fill AGENT_ARGV; launch_agent runs it. Consumers that
# need the driver binary path only (e.g. the reviewer-presence gate) may read
# agent_bin exported by the driver.

set -euo pipefail

RUNTIME_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# resolve_rep_root(): print the repository root that owns this runtime.
resolve_rep_root() {
    echo "$(cd "$RUNTIME_DIR/../../.." && pwd)"
}

# resolve_active_agent is defined here so this file is self-bootstrapping when
# sourced by name (e.g. from run-benchmark.sh). It is also the only consumer
# that must point at a runtime; people source this file directly.
source "$RUNTIME_DIR/agent-env.sh"
if ! resolve_active_agent "$RUNTIME_DIR"; then
    exit $?
fi
source "$RUNTIME_DIR/$AGENT_DRIVER/agent.sh"

# agent_resolve_model(): print the resolved model for the active agent.
# Reads the driver-defined agent_model_env over the agent_default_model.
agent_resolve_model() {
    local env_value
    env_value="${!agent_model_env:-}"
    if [ -n "$env_value" ]; then
        printf '%s' "$env_value"
    else
        printf '%s' "${agent_default_model:-}"
    fi
}

# agent_available(): 0 if the active driver's launch binary is on PATH.
agent_available() {
    [ -n "${agent_bin:-}" ] && command -v "$agent_bin" >/dev/null 2>&1
}

# launch_agent <mode> <timeout|''> <output|'-'>
#   mode:   'setsid'     -> setsid + (optional) timeout, records process group.
#           'background' -> plain background run, no setsid and NO timeout
#                           (preserves the pre-refactor analyzer semantics).
#   timeout: for mode=setsid (e.g. '45m'); ignored for 'background'.
#   output:  path to redirect stdout+stderr to, or '-' for inherit.
# Runs "${AGENT_ARGV[@]}" (set by a driver argv function first) in the
# background, records AGENT_PID and (for setsid) AGENT_PGID.
launch_agent() {
    local mode="$1" timeout="$2" output="$3"
    local -a cmd=()
    if [ "$mode" = "setsid" ]; then
        cmd+=(setsid)
        if [ -n "$timeout" ]; then
            cmd+=(timeout "$timeout")
        fi
    fi
    cmd+=("${AGENT_ARGV[@]}")
    if [ "$output" = "-" ]; then
        "${cmd[@]}" &
    else
        "${cmd[@]}" > "$output" 2>&1 &
    fi
    AGENT_PID="$!"
    AGENT_PGID=""
    if command -v ps >/dev/null 2>&1; then
        AGENT_PGID="$(ps -o pgid= -p "$AGENT_PID" 2>/dev/null | tr -d ' ' || true)"
    fi
}

# wait_agent(): wait for AGENT_PID, store exit code in AGENT_EXIT, clear PID.
wait_agent() {
    if wait "$AGENT_PID"; then
        AGENT_EXIT=0
    else
        AGENT_EXIT=$?
    fi
    AGENT_PID=""
    AGENT_PGID=""
}

# persona_id_for <spawn-role> -> canonical persona id, mirroring the persona
# map in the repo brainstorm (.plans/worker-personas/brainstorm.md). This is
# the single routing table the benchmark dispatch uses so every agent spawn
# assumes a named identity. Accepts both the spawn-role name and the persona
# id; unknown values fail closed (UNKNOWN), which the caller turns into a loud
# identity error via ROLE_ID gating in role-context.sh.
persona_id_for() {
    local key="$1"
    case "$key" in
        worker|benny)                    printf 'benny\n' ;;
        reviewer-a|reviewer_a|christian) printf 'christian\n' ;;
        reviewer-b|reviewer_b|christoph) printf 'christoph\n' ;;
        analyzer|alex)                   printf 'alex\n' ;;
        oracle|pythia)                printf 'oracle\n' ;;
        post-run|postrun|frank|cleanup)  printf 'frank\n' ;;
        *)                               printf 'UNKNOWN\n' ;;
    esac
}

# persona_bootstrap <spawn-role> [output:variables-file]
# Sets ROLE_ID for the named spawn role in the current shell scope. With a
# variables-file argument it writes ROLE_ID=... there (for the variables-file
# pattern used elsewhere) instead of exporting. Unknown roles fail closed by
# setting ROLE_ID=UNKNOWN so downstream ROLE_ID gating refuses the spawn.
persona_bootstrap() {
    local role="$1" id out_file="${2:-}"
    id="$(persona_id_for "$role")"
    [ "$id" = UNKNOWN ] && { printf 'persona: unknown spawn role "%s" (no persona mapping)\n' "$role" >&2; ROLE_ID=UNKNOWN; export ROLE_ID; return 64; }
    if [ -n "$out_file" ]; then
        printf 'ROLE_ID=%s\n' "$id" > "$out_file"
    else
        ROLE_ID="$id"
        export ROLE_ID
    fi
}

# persona_voice <role_id> [VOICES.md path]
# Extracts just the persona's short voice/stance line from roles/VOICES.md,
# keyed by role id. This is the identity preamble the brainstorm calls for
# ("a short preamble injected at spawn"); it never dumps the scoped docs into
# the spawned prompt (those are loaded via role-context.sh by the agent).
persona_voice() {
    local id="$1" file="${2:-}"
    [ -n "$file" ] && [ -f "$file" ] || return 0
    awk -F'|' -v wanted="$id" '
        function trim(v){gsub(/^[[:space:]]+|[[:space:]]+$/,"",v); return v}
        /^\|/ {
            rid=trim($2); gsub(/^`|`$/,"",rid)
            if (rid==wanted) { print trim($3); exit }
        }' "$file"
}

# persona_bootstrap_prompt <prompt-file> <spawn-role> [VOICES.md path]
# Prepends the persona's short voice/stance as an identity preamble to a spawn
# prompt file in place (W09: benchmark spawn sites carry ROLE_ID + role-context
# + voice). The preamble is the single voice line — never the scoped-docs dump,
# so graded benchmark prompts are not perturbed with large content.
persona_bootstrap_prompt() {
    local prompt_file="$1" role="$2" voices="${3:-}"
    local id voice tmp
    id="$(persona_id_for "$role")"
    [ "$id" = UNKNOWN ] && { printf 'persona: unknown spawn role "%s"\n' "$role" >&2; return 64; }
    voice="$(persona_voice "$id" "$voices")"
    [ -f "$prompt_file" ] || { printf 'persona: prompt file missing: %s\n' "$prompt_file" >&2; return 66; }
    tmp="$(mktemp)"
    trap 'rm -f "$tmp"' RETURN
    {
        printf '[PERSONA] id=%s ROLE_ID=%s\n' "$id" "$id"
        [ -n "$voice" ] && printf 'Voice: %s\n' "$voice"
        printf '\n'
        cat "$prompt_file"
    } > "$tmp"
    mv "$tmp" "$prompt_file"
    trap - RETURN
}

# kill_process_tree <pid> <signal>
kill_process_tree() {
    local pid="$1"
    local signal="${2:-TERM}"
    local child

    [ "$pid" -gt 0 ] 2>/dev/null || return 0
    if command -v ps >/dev/null 2>&1; then
        while read -r child; do
            [ -n "$child" ] || continue
            kill_process_tree "$child" "$signal"
        done < <(ps -eo pid=,ppid= | awk -v parent="$pid" '$2 == parent {print $1}')
    fi
    kill -"$signal" "$pid" 2>/dev/null || true
}