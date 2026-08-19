#!/usr/bin/env bash
# Safeguard contract tests for the benchmark harness.
#
# Asserts the process-control, publication-boundary and redaction contracts are
# still visible in the harness source. The generated-case source now spans
# setup-benchmark.sh *and* the extracted benchmark/planning/case/*.sh, so the
# harness-wide assertions search both (harness_grep) rather than one file.
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
runner="$repo_dir/benchmark/planning/run-benchmark.sh"
setup="$repo_dir/benchmark/planning/setup-benchmark.sh"
runtime="$repo_dir/benchmark/planning/runtime"

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
# it: the canonical definition lives in runtime/lib-agent.sh, which the runner
# sources. (This assertion used to accept the runner's own third verbatim copy.)
grep -Fq 'kill_process_tree "$pid" TERM' "$runner"
grep -Fq 'kill_process_tree() {' "$runtime/lib-agent.sh"
if grep -Fq 'kill_process_tree() {' "$runner"; then
    echo 'run-benchmark.sh redefines kill_process_tree instead of using lib-agent.sh' >&2
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
# setsid/timeout are Linux/util-linux+GNU tools: the launcher must refuse loudly
# on a host without them (stock macOS has neither, and ships GNU timeout as
# gtimeout via coreutils) rather than abort the case through set -e.
grep -Fq 'command -v setsid' "$runtime/lib-agent.sh"
grep -Fq 'gtimeout' "$runtime/lib-agent.sh"
# setsid forks when it already leads a process group, so $! can be a parent that
# exits 0 while the agent still runs; --wait keeps AGENT_EXIT the agent's.
grep -Fq 'cmd+=(--wait)' "$runtime/lib-agent.sh"
harness_grep 'launch_agent setsid "${WORKER_TIMEOUT:-45m}"'
harness_grep 'launch_agent setsid "${REVIEWER_TIMEOUT:-20m}"'
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
