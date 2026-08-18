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
safeguard_bin="$(mktemp -d "${TMPDIR:-/tmp}/safeguard-bin.XXXXXX")"
trap 'rm -rf "$safeguard_bin"' EXIT
for stub in setsid timeout gtimeout; do
    printf '#!/usr/bin/env bash\nprintf "%%s\\n" "${0##*/}" >>"$SAFEGUARD_LOG"\nexec "$@"\n' \
        >"$safeguard_bin/$stub"
    chmod +x "$safeguard_bin/$stub"
done
(
    export SAFEGUARD_LOG="$safeguard_bin/log"
    : >"$SAFEGUARD_LOG"
    PATH="$safeguard_bin:$PATH"
    # shellcheck source=/dev/null
    . "$runtime/lib-agent.sh" 2>/dev/null || true
    AGENT_ARGV=(true)
    launch_agent background "" - >/dev/null 2>&1 || true
    wait_agent >/dev/null 2>&1 || true
    if grep -q . "$SAFEGUARD_LOG" 2>/dev/null; then
        printf 'safeguards: background mode invoked %s; it must use neither setsid nor timeout\n' \
            "$(tr '\n' ' ' <"$SAFEGUARD_LOG")" >&2
        exit 1
    fi
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
grep -Fq 'launch_agent background ""' "$runner"
harness_refute 'setsid timeout "${WORKER_TIMEOUT'
harness_refute 'codex -a never exec'
if grep -Fq 'codex -a never exec' "$runner"; then
    echo 'inline codex exec launch still present in the runner' >&2
    exit 1
fi

printf '%s\n' 'Safeguard contract tests passed.'
