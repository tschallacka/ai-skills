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
# agent_argv_analyzer fill AGENT_ARGV (and AGENT_CWD, the directory the agent
# must run in); launch_agent runs it. Consumers that need the driver binary
# path only (e.g. the reviewer-presence gate) may read agent_bin exported by
# the driver.
#
# Resolution failure is fatal: a driver-less runtime cannot launch anything, so
# this file propagates resolve_active_agent's status with exit rather than
# continuing (it is sourced, so exit is the only way to stop the caller).

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
# Inside `if ! cmd; then exit $?` the status is the *negated* pipeline's, i.e.
# 0 — which silently turned an unresolvable agent into a successful no-op run.
# `|| exit "$?"` keeps agent-env.sh's real code (64).
resolve_active_agent "$RUNTIME_DIR" || exit "$?"
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

# _agent_setsid_supports_wait(): 0 when the installed setsid accepts --wait.
# Probed once and cached; util-linux has it, other implementations may not.
_agent_setsid_supports_wait() {
    local probe
    if [ -z "${_AGENT_SETSID_WAIT:-}" ]; then
        probe="$(setsid --help 2>&1 || true)"
        _AGENT_SETSID_WAIT=no
        case "$probe" in
            *--wait*) _AGENT_SETSID_WAIT=yes ;;
        esac
    fi
    [ "$_AGENT_SETSID_WAIT" = yes ]
}

# _agent_capture_pgid <pid>: print the pgid of the launched session, or return 1.
# setsid exec'd -> <pid> leads the group; setsid --wait forked -> <pid> stays in
# the caller's group and its child leads one.
_agent_capture_pgid() {
    local pid="$1" own child
    own="$(ps -o pgid= -p "$pid" 2>/dev/null | tr -d ' ' || true)"
    if [ -n "$own" ] && [ "$own" = "$pid" ]; then
        printf '%s\n' "$own"
        return 0
    fi
    child="$(ps -eo pid=,ppid=,pgid= 2>/dev/null |
        awk -v parent="$pid" '$2 == parent && $1 == $3 { print $3; exit }' || true)"
    [ -n "$child" ] || return 1
    printf '%s\n' "$child"
}

# Runs AGENT_ARGV from AGENT_CWD in the background, setting AGENT_PID/AGENT_PGID.
# Exits 69 when setsid/timeout are required but absent, 70 on empty argv.
# ---- quoted: launch_agent arguments ----
# launch_agent <mode> <timeout|''> <output|'-'>
#   mode=setsid      setsid --wait + timeout, records the process group
#   mode=background  plain background run, no process group of its own
#   timeout          e.g. '45m'; bounds the agent in BOTH modes, '' for unbounded
#   output           path for stdout+stderr, or '-' to inherit
# ---- end quoted ----
# The timeout is orthogonal to the process group: an unbounded background agent
# whose exit code gates a batch hangs that batch forever, so every caller that
# waits on a model passes one.
launch_agent() {
    local mode="$1" timeout_spec="$2" output="$3"
    local -a cmd=()
    local timeout_bin="" cwd="${AGENT_CWD:-$PWD}" isolation_mode="$mode"

    if [ -z "${AGENT_ARGV[*]+set}" ] || [ "${#AGENT_ARGV[@]}" -eq 0 ]; then
        printf 'lib-agent: AGENT_ARGV is empty; call a driver agent_argv_* first\n' >&2
        return 70
    fi
    [ -d "$cwd" ] || { printf 'lib-agent: agent cwd is not a directory: %s\n' "$cwd" >&2; return 66; }

    if [ "$mode" = isolated ]; then
        if command -v setsid >/dev/null 2>&1; then
            isolation_mode=setsid
        else
            isolation_mode=registry
        fi
    fi

    if [ "$isolation_mode" = "setsid" ]; then
        # setsid and timeout are Linux/util-linux+GNU tools. Stock macOS has
        # neither; coreutils installs GNU timeout as gtimeout.
        if ! command -v setsid >/dev/null 2>&1; then
            printf 'lib-agent: setsid not found; process-group isolation is required for mode=setsid (install util-linux, or run the analyzer-style background mode)\n' >&2
            return 69
        fi
        cmd+=(setsid)
        # setsid forks instead of exec'ing when it already leads a process
        # group; without --wait $! can be a parent that exits 0 immediately, so
        # wait_agent would record AGENT_EXIT=0 for a still-running agent.
        if _agent_setsid_supports_wait; then
            cmd+=(--wait)
        else
            printf 'lib-agent: setsid has no --wait; AGENT_EXIT may report the setsid parent rather than the agent\n' >&2
        fi
    fi
    if [ -n "$timeout_spec" ]; then
        if command -v timeout >/dev/null 2>&1; then
            timeout_bin="timeout"
        elif command -v gtimeout >/dev/null 2>&1; then
            timeout_bin="gtimeout"
        else
            if [ "$isolation_mode" = registry ]; then
                printf 'lib-agent: neither timeout nor gtimeout found; registry fallback cannot bound the agent to %s\n' "$timeout_spec" >&2
            else
                printf 'lib-agent: neither timeout nor gtimeout found; cannot bound the agent to %s (install GNU coreutils)\n' "$timeout_spec" >&2
                return 69
            fi
        fi
        [ -z "$timeout_bin" ] || cmd+=("$timeout_bin" "$timeout_spec")
    fi
    cmd+=("${AGENT_ARGV[@]}")

    # The subshell sets the agent's cwd portably: `env -C` is GNU-only and no
    # driver flag exists for every CLI. exec keeps $! pointing at the agent.
    if [ "$output" = "-" ]; then
        ( cd "$cwd" && if [ "$isolation_mode" = registry ] && [ -n "${BENCHMARK_NO_DETACH_SHIM_DIR:-}" ]; then PATH="$BENCHMARK_NO_DETACH_SHIM_DIR:$PATH"; export PATH; fi; exec "${cmd[@]}" ) &
    else
        ( cd "$cwd" && if [ "$isolation_mode" = registry ] && [ -n "${BENCHMARK_NO_DETACH_SHIM_DIR:-}" ]; then PATH="$BENCHMARK_NO_DETACH_SHIM_DIR:$PATH"; export PATH; fi; exec "${cmd[@]}" ) > "$output" 2>&1 &
    fi
    AGENT_PID="$!"
    AGENT_ISOLATION_MODE="$isolation_mode"
    if [ "$isolation_mode" = registry ] && [ -n "${BENCHMARK_PROCESS_REGISTRY:-}" ] && command -v process_registry_append >/dev/null 2>&1; then
        process_registry_append "$BENCHMARK_PROCESS_REGISTRY" agent "$AGENT_PID" "$output" "${cmd[@]}"
    fi

    # Polled, not read once: a not-yet-scheduled agent has no observable group,
    # and an empty AGENT_PGID silently disables the process-group kill.
    AGENT_PGID=""
    if [ "$isolation_mode" = setsid ] && command -v ps >/dev/null 2>&1; then
        local attempt=0
        while [ "$attempt" -lt 20 ]; do
            AGENT_PGID="$(_agent_capture_pgid "$AGENT_PID" || true)"
            [ -z "$AGENT_PGID" ] || break
            kill -0 "$AGENT_PID" 2>/dev/null || break
            sleep 0.1
            attempt=$((attempt + 1))
        done
        if [ -z "$AGENT_PGID" ]; then
            printf 'lib-agent: could not observe a process group for agent pid %s (it may have exited immediately)\n' "$AGENT_PID" >&2
        fi
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

# persona_id_for <spawn-role|persona-id> -> canonical persona id.
# Unknown values fail closed as UNKNOWN, which ROLE_ID gating turns into a loud
# identity error rather than an anonymous spawn.
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
# Exports ROLE_ID, or writes ROLE_ID=... to the variables file when given one.
# An unknown role sets ROLE_ID=UNKNOWN so downstream gating refuses the spawn.
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
# Extracts only the persona's short voice/stance line; the scoped role docs are
# deliberately never pulled in here.
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
# Prepends the identity preamble to the prompt file in place. The preamble is
# the single voice line only, so a graded prompt is not perturbed by bulk text.
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

# benchmark_latest_tag(): latest git tag reachable from HEAD, or "-" so the
# results path stays valid when it is unresolvable. Resolves the repo root
# itself, so it works whether or not the caller set REPO_ROOT.
benchmark_latest_tag() {
    local tag repo
    if [ -n "${REPO_ROOT:-}" ]; then
        repo="$REPO_ROOT"
    else
        repo="$(cd "$RUNTIME_DIR/../../.." && pwd)"
    fi
    tag="$(git -C "$repo" describe --tags --abbrev=0 HEAD 2>/dev/null || true)"
    [ -n "$tag" ] || tag="-"
    printf '%s\n' "$tag"
}

# benchmark_result_parent <tag>: the subpath under results/<agent>/ for <tag> —
# the bare tag, or current/<latest-tag> for the reserved `current` tag.
benchmark_result_parent() {
    local tag="$1"
    if [ "$tag" = current ]; then
        printf 'current/%s\n' "$(benchmark_latest_tag)"
    else
        printf '%s\n' "${tag#v}"
    fi
}
