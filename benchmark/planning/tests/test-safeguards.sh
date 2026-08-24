#!/usr/bin/env bash
# Safeguard contract tests for the benchmark harness.
#
# Asserts the process-control, publication-boundary and redaction contracts are
# still visible in the harness source. The generated-case source now spans
# setup-benchmark.sh *and* the extracted benchmark/planning/case/*.sh, so the
# harness-wide assertions search both (harness_grep) rather than one file.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../planning/tests" && pwd)/lib-test.sh"
# Report the failing expression: these assertions are bare `[ ... ]` under
# set -e, which otherwise exits 1 in silence.
t_trap_assertions


repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
runner="$repo_dir/benchmark/planning/run-benchmark.sh"
setup="$repo_dir/benchmark/planning/setup-benchmark.sh"
runtime="$repo_dir/benchmark/planning/runtime"
process_lib="$repo_dir/benchmark/planning/lib-process.sh"

harness_sources=("$setup")
for case_source in "$repo_dir/benchmark/planning/case"/*.sh; do
    [ -f "$case_source" ] && harness_sources+=("$case_source")
done

# harness_grep <fixed-pattern>: 0 when any harness source carries it.
harness_grep() {
    local pattern="$1" source
    for source in "${harness_sources[@]}"; do
        grep -Fq "$pattern" "$source" && return 0
    done
    echo "harness source lost the safeguard: $pattern" >&2
    return 1
}

# harness_refute <fixed-pattern>: 0 when no harness source carries it.
harness_refute() {
    local pattern="$1" source
    for source in "${harness_sources[@]}"; do
        if grep -Fq "$pattern" "$source"; then
            echo "harness source regressed: $pattern in $source" >&2
            return 1
        fi
    done
    return 0
}

grep -Fq 'trap cleanup_on_signal INT TERM' "$runner"
# The runner must *call* kill_process_tree, and must not carry its own copy of
# it: the canonical definition lives in lib-process.sh, which runtime/lib-agent.sh
# sources through to the runner. (This assertion used to accept the runner's own
# third verbatim copy, and until T32 both libraries still carried a twin.)
grep -Fq 'kill_process_tree "$pid" TERM' "$runner"
grep -Fq 'kill_process_tree() {' "$process_lib"
grep -Fq 'lib-process.sh' "$runtime/lib-agent.sh"
if grep -Fq 'kill_process_tree() {' "$runner"; then
    echo 'run-benchmark.sh redefines kill_process_tree instead of using lib-process.sh via lib-agent.sh' >&2
    exit 1
fi
# Substring, not the whole line: the generated case's handler has been renamed
# (cleanup_on_signal / process_cleanup_on_signal) while the contract — a signal
# trap that tears the worker's process group down — is unchanged.
harness_grep 'cleanup_on_signal INT TERM'
harness_grep 'WORKER_PROCESS_GROUP_ID'
harness_grep 'STAGING_RESULT_DIR'
harness_grep 'copy_workspace_for_publication() {'
harness_grep 'mv "$STAGING_RESULT_DIR" "$RESULT_DIR"'
harness_grep 'STATUS="tainted"'
harness_grep 'No compatibility fallback is permitted'
harness_grep 'refusing compatibility patch'
harness_refute 'SKIPPED (fixture unavailable'
grep -Fq 'telemetry-schema.json' "$repo_dir/benchmark/planning/tests/test-telemetry-integrity.sh"

# The publication boundary is part of the protocol, not merely an archive
# implementation detail. Keep the redaction contract visible in the source
# and fail the suite if a published report can regress to private identifiers.
oracle="$repo_dir/benchmark/planning/review-oracle.sh"
grader="$repo_dir/benchmark/planning/grade-blinded-run.sh"
grep -Fq 'redacted' "$oracle"
grep -Fq 'defect_id' "$oracle"
grep -Fq 'mutation' "$oracle"
grep -Fq '<private>' "$oracle"
harness_grep 'fail_closed_reasons'
harness_grep 'adoptable'
grep -Fq 'oracle.json' "$grader"

if grep -R --line-number --include='*.json' --include='*.md' \
    'ai-skills-oracle-private\|oracle-key\|defect-map.enc' \
    "$repo_dir/benchmark/planning/tests/fixtures" >/dev/null 2>&1; then
    echo 'private oracle material leaked into public fixtures' >&2
    exit 1
fi

# Process-control split: lib-agent.sh owns setsid/timeout/background argv
# execution, start-worker keeps worker isolation, run-benchmark.sh keeps
# batch-level cleanup. No launch path may hardcode a bare inline `codex exec`.
grep -Fq 'launch_agent()' "$runtime/lib-agent.sh"
grep -Fq 'cmd+=(setsid)' "$runtime/lib-agent.sh"
grep -Fq 'cmd+=("$timeout_bin" "$timeout_spec")' "$runtime/lib-agent.sh"
grep -Fq 'isolation_mode=registry' "$runtime/lib-agent.sh"
grep -Fq 'BENCHMARK_NO_DETACH_SHIM_DIR' "$runtime/lib-agent.sh"
grep -Fq 'process_registry_append "$BENCHMARK_PROCESS_REGISTRY"' "$runtime/lib-agent.sh"
grep -Fq 'background' "$runtime/lib-agent.sh"
# Behavioural, not textual: this used to grep for the phrase "no setsid" in a
# comment, so it passed on prose and failed on rewording while proving nothing.
# Background mode owns no process group, but a timeout is orthogonal to that and
# must still be applied — the analyzer gates the batch exit code.
safeguard_bin="$(mktemp -d "${TMPDIR:-/tmp}/safeguard-bin.XXXXXX")"
trap 'rm -rf "$safeguard_bin"' EXIT
for stub in setsid timeout gtimeout; do
    printf '#!/usr/bin/env bash\nprintf "%%s\\n" "${0##*/}" >>"$SAFEGUARD_LOG"\nexec "$@"\n' \
        >"$safeguard_bin/$stub"
    chmod +x "$safeguard_bin/$stub"
done
safeguard_background_launch() {
    export SAFEGUARD_LOG="$safeguard_bin/log"
    : >"$SAFEGUARD_LOG"
    PATH="$safeguard_bin:$PATH"
    # shellcheck source=/dev/null
    . "$runtime/lib-agent.sh" 2>/dev/null || true
    AGENT_ARGV=(true)
    launch_agent background "$1" - >/dev/null 2>&1 || true
    wait_agent >/dev/null 2>&1 || true
    tr '\n' ' ' <"$SAFEGUARD_LOG"
}
(
    invoked="$(safeguard_background_launch '')"
    case "$invoked" in
        '') ;;
        *)
            printf 'safeguards: unbounded background mode invoked %s; an empty timeout must wrap nothing\n' "$invoked" >&2
            exit 1
            ;;
    esac
)
(
    invoked="$(safeguard_background_launch 30m)"
    case "$invoked" in
        *setsid*)
            printf 'safeguards: background mode invoked setsid (%s); it must claim no process group\n' "$invoked" >&2
            exit 1
            ;;
    esac
    case "$invoked" in
        *timeout*) ;;
        *)
            printf 'safeguards: background mode with a timeout invoked "%s"; the bound must be applied, not dropped\n' "$invoked" >&2
            exit 1
            ;;
    esac
)
# setsid/timeout are Linux/util-linux+GNU tools. Worker/reviewer launches use
# isolated mode: setsid stays the strong path when present, and registry fallback
# is used only when setsid is unavailable.
grep -Fq 'command -v setsid' "$runtime/lib-agent.sh"
grep -Fq 'gtimeout' "$runtime/lib-agent.sh"
grep -Fq 'process_prepare_no_detach_shims "$BENCHMARK_NO_DETACH_SHIM_DIR"' "$repo_dir/benchmark/planning/case/start-worker.sh"
grep -Fq 'if ! command -v setsid >/dev/null 2>&1; then' "$repo_dir/benchmark/planning/case/start-worker.sh"
if grep -R --line-number 'os\.setsid' "$repo_dir/benchmark/planning/tests" >/dev/null 2>&1; then
    echo 'benchmark tests reintroduced a Python fake setsid shim' >&2
    exit 1
fi
# shellcheck source=../lib-process.sh
source "$process_lib"
registry_probe="$(mktemp -d "${TMPDIR:-/tmp}/benchmark-registry.XXXXXX")"
registry_file="$registry_probe/process-registry.tsv"
process_registry_init "$registry_file"
process_registry_append "$registry_file" worker 12345 worker.jsonl codex exec --dangerously-bypass-approvals-and-sandbox
test -s "$registry_file"
awk -F "$(printf '\t')" '
    $1 == "worker" && $2 == "12345" && $5 == "worker.jsonl" && $6 ~ /codex exec/ { found = 1 }
    END { exit(found ? 0 : 1) }
' "$registry_file"

signal_probe_child="$registry_probe/child.pid"
"$BASH" -c '
set -euo pipefail
sleep 60 &
printf "%s\n" "$!" > "$1"
wait
' _ "$signal_probe_child" &
signal_probe_root="$!"
signal_wait=0
while [ ! -s "$signal_probe_child" ] && [ "$signal_wait" -lt 50 ]; do
    kill -0 "$signal_probe_root" 2>/dev/null || break
    sleep 0.1
    signal_wait=$((signal_wait + 1))
done
test -s "$signal_probe_child"
signal_probe_descendant="$(cat "$signal_probe_child")"
if (
    set +E
    trap - ERR
    PROCESS_CLEANUP_CHILD_PID="$signal_probe_root"
    PROCESS_CLEANUP_GROUP_ID=""
    process_cleanup_on_signal
); then
    signal_status=0
else
    signal_status="$?"
fi
if [ "$signal_status" -ne 130 ]; then
    printf 'safeguards: process_cleanup_on_signal exited %s, expected 130\n' "$signal_status" >&2
    exit 1
fi
wait "$signal_probe_root" 2>/dev/null || true
signal_wait=0
while { kill -0 "$signal_probe_root" 2>/dev/null || kill -0 "$signal_probe_descendant" 2>/dev/null; } &&
    [ "$signal_wait" -lt 20 ]; do
    kill_process_tree "$signal_probe_root" KILL
    sleep 0.1
    signal_wait=$((signal_wait + 1))
done
if kill -0 "$signal_probe_root" 2>/dev/null || kill -0 "$signal_probe_descendant" 2>/dev/null; then
    printf 'safeguards: signal cleanup left registered process tree running: root=%s child=%s\n' "$signal_probe_root" "$signal_probe_descendant" >&2
    exit 1
fi

shim_probe="$(mktemp -d "${TMPDIR:-/tmp}/benchmark-no-detach.XXXXXX")"
process_prepare_no_detach_shims "$shim_probe"
for detach_tool in nohup setsid open; do
    if "$shim_probe/$detach_tool" true >/dev/null 2>"$shim_probe/$detach_tool.err"; then
        printf 'safeguards: %s shim allowed a detached command under registry fallback\n' "$detach_tool" >&2
        exit 1
    fi
    grep -Fq 'detached subprocess launch is not allowed under registry isolation' "$shim_probe/$detach_tool.err"
done
rm -rf "$shim_probe"
rm -rf "$registry_probe"

# B13: with neither timeout nor gtimeout the registry fallback used to warn and
# then launch a real, unbounded agent. The bound is a shell watchdog now, and
# these assertions are behavioural — on fake agents, never a real one.
watchdog_probe="$(mktemp -d "${TMPDIR:-/tmp}/benchmark-watchdog.XXXXXX")"
watchdog_bin="$watchdog_probe/bin"
mkdir -p "$watchdog_bin"
# The launch PATH carries only what the launcher and the watchdog use, so
# `command -v timeout` fails exactly as it does on stock macOS.
for watchdog_tool in ps awk tr sleep; do
    ln -s "$(command -v "$watchdog_tool")" "$watchdog_bin/$watchdog_tool"
done
cat > "$watchdog_probe/slow-agent" <<'AGENT'
sleep 300 &
printf '%s\n' "$!" > "$WATCHDOG_CHILD_PID_FILE"
wait
AGENT
cat > "$watchdog_probe/quick-agent" <<'AGENT'
exit 0
AGENT
cat > "$watchdog_probe/deaf-agent" <<'AGENT'
trap '' TERM
sleep 300 &
printf '%s\n' "$!" > "$WATCHDOG_CHILD_PID_FILE"
wait
AGENT
# Stubs are #!$BASH, not #!/usr/bin/env bash: the launch PATH has no bash.
watchdog_timeout_bin="$watchdog_probe/timeout-bin"
mkdir -p "$watchdog_timeout_bin"
{
    printf '#!%s\n' "$BASH"
    cat <<'STUB'
printf '%s\n' "${0##*/}" >> "$WATCHDOG_LADDER_LOG"
shift
exec "$@"
STUB
} > "$watchdog_timeout_bin/timeout"
chmod +x "$watchdog_timeout_bin/timeout"
watchdog_setsid_bin="$watchdog_probe/setsid-bin"
mkdir -p "$watchdog_setsid_bin"
{
    printf '#!%s\n' "$BASH"
    cat <<'STUB'
case "${1:-}" in
    --help) printf '%s\n' --wait; exit 0 ;;
esac
exec "$@"
STUB
} > "$watchdog_setsid_bin/setsid"
chmod +x "$watchdog_setsid_bin/setsid"

# watchdog_launch <action> <mode> <spec> <agent-script> <fact-dir> [extra-bin]
# A subshell cannot return values, so every fact lands in a file under
# <fact-dir>. action=detach leaves the launch running for the caller to observe,
# wait reaps it through wait_agent, selfexit watches the watchdog stop itself.
watchdog_launch() {
    local action="$1" mode="$2" spec="$3" script="$4" facts="$5" extra_bin="${6:-}"
    mkdir -p "$facts"
    (
        set +E
        trap - ERR
        # shellcheck source=/dev/null
        . "$runtime/lib-agent.sh" 2>/dev/null || true
        PATH="$watchdog_bin"
        [ -z "$extra_bin" ] || PATH="$extra_bin:$PATH"
        export PATH
        export WATCHDOG_CHILD_PID_FILE="$facts/child.pid"
        export WATCHDOG_LADDER_LOG="$facts/ladder.log"
        : > "$WATCHDOG_LADDER_LOG"
        AGENT_ARGV=("$BASH" "$script")
        if launch_agent "$mode" "$spec" "$facts/agent.log"; then
            printf '0\n' > "$facts/status"
        else
            printf '%s\n' "$?" > "$facts/status"
        fi
        printf '%s\n' "${AGENT_PID:-}" > "$facts/agent.pid"
        printf '%s\n' "${AGENT_WATCHDOG_PID:-}" > "$facts/watchdog.pid"
        watchdog_observe "$action" "$facts"
    ) 2> "$facts/err"
}

# watchdog_observe <action> <fact-dir>: what happened after the launch. Called
# inside watchdog_launch's subshell, so it reads the live AGENT_* state.
watchdog_observe() {
    local action="$1" facts="$2" guard=0 watchdog_pid="${AGENT_WATCHDOG_PID:-}"
    if [ "$action" = selfexit ]; then
        while [ -n "$watchdog_pid" ] && kill -0 "$watchdog_pid" 2>/dev/null &&
            [ "$guard" -lt 50 ]; do
            sleep 0.2
            guard=$((guard + 1))
        done
        if [ -n "$watchdog_pid" ] && kill -0 "$watchdog_pid" 2>/dev/null; then
            printf 'no\n' > "$facts/selfexit"
        else
            printf 'yes\n' > "$facts/selfexit"
        fi
    fi
    [ "$action" != detach ] && [ -n "${AGENT_PID:-}" ] || return 0
    # Recorded before the teardown: an orphaned sleep is reparented to init, so
    # afterwards nothing links it back to the watchdog that started it.
    sleep 0.3
    process_descendants "$watchdog_pid" > "$facts/watchdog-children" || true
    wait_agent || true
    printf '%s\n' "${AGENT_EXIT:-}" > "$facts/exit"
    if [ -n "$watchdog_pid" ] && kill -0 "$watchdog_pid" 2>/dev/null; then
        printf 'alive\n' > "$facts/watchdog-after"
    else
        printf 'gone\n' > "$facts/watchdog-after"
    fi
}

# 1 and 3. An agent that outlives its bound is killed, and so is its descendant:
# registry mode owns no process group, so the ps walk is the only reach it has.
watchdog_kill="$watchdog_probe/kill"
watchdog_launch detach isolated 2s "$watchdog_probe/slow-agent" "$watchdog_kill"
watchdog_status="$(cat "$watchdog_kill/status")"
if [ "$watchdog_status" != 0 ]; then
    printf 'safeguards: registry launch with no timeout binary returned %s (err: %s)\n' \
        "$watchdog_status" "$(tr '\n' ' ' < "$watchdog_kill/err")" >&2
    exit 1
fi
watchdog_agent="$(cat "$watchdog_kill/agent.pid")"
watchdog_pid="$(cat "$watchdog_kill/watchdog.pid")"
[ -n "$watchdog_agent" ]
if [ -z "$watchdog_pid" ]; then
    printf 'safeguards: registry mode launched agent pid %s with no watchdog; the run is unbounded (B13)\n' "$watchdog_agent" >&2
    exit 1
fi
watchdog_wait=0
while [ ! -s "$watchdog_kill/child.pid" ] && [ "$watchdog_wait" -lt 100 ]; do
    sleep 0.1
    watchdog_wait=$((watchdog_wait + 1))
done
test -s "$watchdog_kill/child.pid"
watchdog_child="$(cat "$watchdog_kill/child.pid")"
watchdog_wait=0
while { kill -0 "$watchdog_agent" 2>/dev/null || kill -0 "$watchdog_child" 2>/dev/null; } &&
    [ "$watchdog_wait" -lt 150 ]; do
    sleep 0.2
    watchdog_wait=$((watchdog_wait + 1))
done
if kill -0 "$watchdog_agent" 2>/dev/null; then
    printf 'safeguards: the watchdog left agent pid %s running past its 2s bound\n' "$watchdog_agent" >&2
    exit 1
fi
if kill -0 "$watchdog_child" 2>/dev/null; then
    printf 'safeguards: the watchdog killed agent %s but left descendant %s running\n' \
        "$watchdog_agent" "$watchdog_child" >&2
    exit 1
fi
grep -Fq 'exceeded its 2s bound' "$watchdog_kill/err"

# An agent that ignores TERM is still bounded: a CLI that traps the signal would
# otherwise turn the escalation into a suggestion.
watchdog_deaf="$watchdog_probe/deaf"
watchdog_launch detach isolated 1s "$watchdog_probe/deaf-agent" "$watchdog_deaf"
[ "$(cat "$watchdog_deaf/status")" = 0 ]
watchdog_agent="$(cat "$watchdog_deaf/agent.pid")"
watchdog_wait=0
while kill -0 "$watchdog_agent" 2>/dev/null && [ "$watchdog_wait" -lt 150 ]; do
    sleep 0.2
    watchdog_wait=$((watchdog_wait + 1))
done
if kill -0 "$watchdog_agent" 2>/dev/null; then
    printf 'safeguards: agent pid %s ignored TERM and survived; the watchdog never escalated to KILL\n' "$watchdog_agent" >&2
    exit 1
fi

# 2. An agent that exits on its own is not killed, and nothing survives it.
watchdog_normal="$watchdog_probe/normal"
watchdog_launch wait isolated 300s "$watchdog_probe/quick-agent" "$watchdog_normal"
[ "$(cat "$watchdog_normal/status")" = 0 ]
[ "$(cat "$watchdog_normal/exit")" = 0 ]
watchdog_pid="$(cat "$watchdog_normal/watchdog.pid")"
[ -n "$watchdog_pid" ]
if grep -Fq 'exceeded its' "$watchdog_normal/err"; then
    printf 'safeguards: the watchdog killed an agent that had already exited\n' >&2
    exit 1
fi
if [ "$(cat "$watchdog_normal/watchdog-after")" != gone ]; then
    printf 'safeguards: watchdog pid %s outlived the agent it was bounding\n' "$watchdog_pid" >&2
    exit 1
fi
watchdog_strays="$(tr '\n' ' ' < "$watchdog_normal/watchdog-children")"
if [ -z "$watchdog_strays" ]; then
    printf 'safeguards: the watchdog had no sleep to clean up, so the stray check proves nothing\n' >&2
    exit 1
fi
for watchdog_stray in $watchdog_strays; do
    if kill -0 "$watchdog_stray" 2>/dev/null; then
        printf 'safeguards: watchdog teardown left sleep pid %s behind\n' "$watchdog_stray" >&2
        exit 1
    fi
done

# The watchdog also stops itself, for a caller that never reaches wait_agent.
watchdog_self="$watchdog_probe/self"
watchdog_launch selfexit isolated 300s "$watchdog_probe/quick-agent" "$watchdog_self"
if [ "$(cat "$watchdog_self/selfexit")" != yes ]; then
    printf 'safeguards: the watchdog kept polling after its agent exited; one long sleep is not a bound that cleans up\n' >&2
    exit 1
fi

# 4. An unparseable bound refuses rather than guessing a unit.
watchdog_spec="$watchdog_probe/spec"
watchdog_launch detach isolated 45x "$watchdog_probe/quick-agent" "$watchdog_spec"
if [ "$(cat "$watchdog_spec/status")" != 64 ]; then
    printf 'safeguards: an unparseable timeout spec returned %s, expected 64\n' "$(cat "$watchdog_spec/status")" >&2
    exit 1
fi
grep -Fq 'cannot parse timeout spec' "$watchdog_spec/err"
[ -z "$(cat "$watchdog_spec/agent.pid")" ]

# 5. Pin the ladder: a real timeout binary is used, and the watchdog is not.
watchdog_ladder="$watchdog_probe/ladder"
watchdog_launch wait isolated 300s "$watchdog_probe/quick-agent" "$watchdog_ladder" "$watchdog_timeout_bin"
[ "$(cat "$watchdog_ladder/status")" = 0 ]
grep -Fq timeout "$watchdog_ladder/ladder.log"
if [ -n "$(cat "$watchdog_ladder/watchdog.pid")" ]; then
    printf 'safeguards: the watchdog ran while timeout was on PATH; it is the last rung, not the default\n' >&2
    exit 1
fi

# 6. The setsid path keeps its refusal: no timeout binary there is still 69.
watchdog_setsid="$watchdog_probe/setsid"
watchdog_launch detach isolated 5s "$watchdog_probe/quick-agent" "$watchdog_setsid" "$watchdog_setsid_bin"
if [ "$(cat "$watchdog_setsid/status")" != 69 ]; then
    printf 'safeguards: the setsid path returned %s with no timeout binary, expected 69\n' "$(cat "$watchdog_setsid/status")" >&2
    exit 1
fi
grep -Fq 'cannot bound the agent to 5s' "$watchdog_setsid/err"
[ -z "$(cat "$watchdog_setsid/watchdog.pid")" ]

for watchdog_leftover in "$watchdog_kill" "$watchdog_deaf" "$watchdog_normal" "$watchdog_self" "$watchdog_ladder"; do
    [ -s "$watchdog_leftover/agent.pid" ] || continue
    kill_process_tree "$(cat "$watchdog_leftover/agent.pid")" KILL
done
rm -rf "$watchdog_probe"
# setsid forks when it already leads a process group, so $! can be a parent that
# exits 0 while the agent still runs; --wait keeps AGENT_EXIT the agent's.
grep -Fq 'cmd+=(--wait)' "$runtime/lib-agent.sh"
harness_grep 'launch_agent isolated "${WORKER_TIMEOUT:-45m}"'
harness_grep 'launch_agent isolated "${REVIEWER_TIMEOUT:-20m}"'
# The analyzer gates the batch exit code, so it carries a bound in the same
# shape as the worker's and the reviewer's. It used to be launched with "".
if ! grep -Fq 'launch_agent background "${ANALYZER_TIMEOUT:-30m}"' "$runner"; then
    echo 'the batch analyzer lost its ANALYZER_TIMEOUT bound; a hung analyzer hangs the whole batch' >&2
    exit 1
fi
# A REVIEWER_COMMAND seam under a non-codex driver must refuse, never fall
# through to the real reviewer and spend model budget inside a test.
harness_grep 'refusing to run the real reviewer in its place'
harness_refute 'running the real driver'
harness_refute 'setsid timeout "${WORKER_TIMEOUT'
harness_refute 'codex -a never exec'
if grep -Fq 'codex -a never exec' "$runner"; then
    echo 'inline codex exec launch still present in the runner' >&2
    exit 1
fi

# Telemetry taint, exercised on the real condition line rather than grepped for:
# "absent" is spelled `unavailable`, so a genuinely reported zero token count is
# a count and must leave an otherwise clean case accepted.
taint_condition="$(grep -F 'TELEMETRY_STATUS" != "available"' \
    "$repo_dir/benchmark/planning/case/start-worker.sh" | head -1)"
case "$taint_condition" in
    if*"; then") ;;
    *)
        echo 'safeguards: could not find the telemetry taint condition in start-worker.sh' >&2
        exit 1
        ;;
esac

# taint_verdict <session-id> <status> <usage-records> <total-usage>
taint_verdict() (
    SESSION_ID="$1"; TELEMETRY_STATUS="$2"; USAGE_RECORDS="$3"; TOTAL_USAGE="$4"
    eval "$taint_condition printf tainted; else printf clean; fi"
)

assert_taint() {
    local expected="$1" label="$2"; shift 2
    local actual
    actual="$(taint_verdict "$@")"
    if [ "$actual" != "$expected" ]; then
        printf 'safeguards: telemetry taint for %s is %s, expected %s\n' "$label" "$actual" "$expected" >&2
        exit 1
    fi
}
assert_taint clean   'a reported zero token count' session-1 available 1 0
assert_taint clean   'a reported token count' session-1 available 3 4096
assert_taint tainted 'an absent token count' session-1 available 3 unavailable
assert_taint tainted 'an empty token count' session-1 available 3 ''
assert_taint tainted 'zero usage records' session-1 available 0 4096
assert_taint tainted 'unavailable telemetry' session-1 'unavailable:no store' 3 4096
assert_taint tainted 'an unavailable session id' unavailable available 3 4096

printf '%s\n' 'Safeguard contract tests passed.'
