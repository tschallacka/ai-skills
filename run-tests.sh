#!/usr/bin/env bash
# MODE: DEV
# Deterministic, repeatable test runner for the whole repo.
#
# Runs every test under tests/, planning/tests/ and benchmark/planning/tests/ in a
# fixed sorted order, each under the resource-limited wrapper, and reports a
# stable summary. The order and per-test result are the same on every run on
# the same host, so CI or a maintainer sees identical output.
#
# Usage: run-tests.sh [--verbose]
#
# A failing test's full output is always printed — it is the only diagnostic the
# runner has, and truncating it to the last 20 lines hid the failing assertion.
# `--verbose` additionally prints the output of tests that passed.
#
# `set -uo pipefail` deliberately omits `-e` (the sanctioned exception in
# CODE-STYLE.md §2): a failing test must not abort the loop before the summary
# is printed. Each test's status is captured explicitly instead.
#
# Suite paths are resolved against the repo root, so the runner works from any
# working directory.
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
wrapper="$repo_root/resource-limited-testing/scripts/limited-run.sh"
verbose=false
[ "${1:-}" = "--verbose" ] && verbose=true

# Scope everything this run creates under one scratch root and clean it up on
# exit so no temp files survive a pass, fail, or interrupted run. The planning
# skill writes scratch under ${TMPDIR}/planning-agent, the benchmark runner
# places capsules under PLANNING_AGENT_TMPDIR, and archives are staged under
# benchmark/results/<agent>/.staging/ — all three are covered below.
run_scratch="$(mktemp -d "${TMPDIR:-/tmp}/ai-skills-tests.XXXXXX")"
test_output="$run_scratch/test-output.txt"
AI_SKILLS_TEST_RUN_ID="run-tests.$$.$(date -u +%s)"
export AI_SKILLS_TEST_RUN_ID
export TMPDIR="$run_scratch"
export PLANNING_AGENT_TMPDIR="$run_scratch/planning-agent"
cleanup_marked_test_roots() {
    local scan candidate marker marker_value
    for scan in /tmp "${TMPDIR:-}"; do
        [ -n "$scan" ] || continue
        [ -d "$scan" ] || continue
        while IFS= read -r candidate; do
            [ -n "$candidate" ] || continue
            marker="$candidate/.ai-skills-test-run-id"
            [ -f "$marker" ] || continue
            marker_value="$(sed -n '1p' "$marker")"
            [ "$marker_value" = "$AI_SKILLS_TEST_RUN_ID" ] || continue
            rm -rf -- "$candidate"
        done < <(find -H "$scan" -maxdepth 1 -type d -name 't.?????' -print 2>/dev/null)
    done
}
cleanup() {
    cleanup_marked_test_roots
    rm -rf -- "$run_scratch"
    # Each test takes a short root directly under /tmp -- lib-test.sh explains
    # why it cannot nest under this scratch. The run-id marker lets this cleanup
    # remove only roots from this suite run, even if a test overwrote lib-test's
    # EXIT trap after sourcing it.
    rm -rf -- "$repo_root/benchmark/results"/*/.staging
}
trap cleanup EXIT

# Tests that require PLANNING_CONTEXT_CACHE (a developer-only legacy context
# cache). They are documented to fail closed when the fixture is absent; the
# runner prefers to run them when the fixture is configured and otherwise
# reports them as UNCONFIGURED rather than as a silent skip or a hard failure.
context_gated=(
    planning/tests/test-plan-context.sh
    planning/tests/test-plan-context-deferred-boundary.sh
)

# $1 is an absolute test path; context_gated lists repo-relative paths.
is_context_gated() {
    local t="${1#"$repo_root"/}" entry
    for entry in "${context_gated[@]}"; do
        [ "$entry" = "$t" ] && return 0
    done
    return 1
}

# Discover test scripts in a suite, sorted for determinism.
discover() {
    local dir="$1"
    find "$dir" -maxdepth 1 -type f -name 'test-*.sh' -print | sort
}

# Rust crates are separate gate cases so a broken crate cannot hide behind the
# shell-suite result. A contributor without cargo gets an explicit, non-failing
# unconfigured result unless the CI refusal switch is set.
discover_crates() {
    [ -f "$repo_root/src/plan-overview/Cargo.toml" ] && printf '%s\n' src/plan-overview
}

suites=(
    tests
    planning/tests
    # chat/tests was never discovered here, so every chat assertion — including
    # the rung CI builds cargo for specifically "so its assertions run" (T62) —
    # was dead weight: a green suite proved nothing about the chat skill.
    chat/tests
    benchmark/planning/tests
)

tests=()
for suite in "${suites[@]}"; do
    while IFS= read -r t; do
        tests+=("$t")
    done < <(discover "$repo_root/$suite")
done

total=0
passed=0
failed=0
unconfigured=0
declare -a failed_names=()
declare -a unconfigured_names=()

run_one() {
    local t="$1" label mem cpu
    label="$(sed 's#^.*/tests/##; s#\.sh$##' <<<"$t")"
    # benchmark tests spin up worker/reviewer-like processes; give them headroom.
    case "$t" in
        "$repo_root"/benchmark/*) mem=6G; cpu=400 ;;
        *) mem=2G; cpu=400 ;;
    esac

    if is_context_gated "$t" && [ -z "${PLANNING_CONTEXT_CACHE:-}" ]; then
        unconfigured=$((unconfigured + 1))
        unconfigured_names+=("$label")
        printf '  %-52s UNCONFIGURED (PLANNING_CONTEXT_CACHE)\n' "$label"
        return
    fi

    total=$((total + 1))
    if "$wrapper" "$mem" "$cpu" -- "$BASH" "$t" >"$test_output" 2>&1; then
        passed=$((passed + 1))
        printf '  %-52s PASS\n' "$label"
        if [ "$verbose" = true ]; then
            sed 's/^/      /' "$test_output"
        fi
    else
        code=$?
        failed=$((failed + 1))
        failed_names+=("$label")
        printf '  %-52s FAIL (exit %s)\n' "$label" "$code"
        # Always the whole output, --verbose or not.
        sed 's/^/      /' "$test_output"
    fi
    rm -f "$test_output"
}

start="$(date -u +%s)"
echo "Testing $(basename "$repo_root") — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Runner order: sorted test files under planning/tests then benchmark/planning/tests"
echo

for t in ${tests[@]+"${tests[@]}"}; do
    run_one "$t"
done

run_cargo_one() {
    local crate="$1" label="cargo-${crate##*/}"
    if ! command -v cargo >/dev/null 2>&1; then
        if [ "${REFUSE_UNCONFIGURED_CARGO:-0}" = 1 ]; then
            failed=$((failed + 1))
            failed_names+=("$label")
            printf '  %-52s FAIL (cargo unavailable; REFUSE_UNCONFIGURED_CARGO=1)\n' "$label"
        else
            unconfigured=$((unconfigured + 1))
            unconfigured_names+=("$label")
            printf '  %-52s UNCONFIGURED (cargo)\n' "$label"
        fi
        return
    fi
    total=$((total + 1))
    if "$wrapper" 2G 400 -- cargo test --manifest-path "$repo_root/$crate/Cargo.toml" >"$test_output" 2>&1; then
        passed=$((passed + 1))
        printf '  %-52s PASS\n' "$label"
        [ "$verbose" = true ] && sed 's/^/      /' "$test_output"
    else
        code=$?
        failed=$((failed + 1))
        failed_names+=("$label")
        printf '  %-52s FAIL (exit %s)\n' "$label" "$code"
        sed 's/^/      /' "$test_output"
    fi
    rm -f "$test_output"
}

while IFS= read -r crate; do
    [ -n "$crate" ] || continue
    run_cargo_one "$crate"
done < <(discover_crates)

elapsed="$(( $(date -u +%s) - start ))"
echo
echo "──────────────────────────────────────────────"
printf 'Total ran: %d   Passed: %d   Failed: %d   Unconfigured: %d\n' "$total" "$passed" "$failed" "$unconfigured"
printf 'Elapsed: %ds\n' "$elapsed"
# bash 3.2 treats "${arr[@]}" of an empty array as unbound under `set -u`.
if [ -n "${failed_names[*]+set}" ] && [ "${#failed_names[@]}" -gt 0 ]; then
    printf 'Failed: %s\n' "${failed_names[*]-}"
fi
if [ -n "${unconfigured_names[*]+set}" ] && [ "${#unconfigured_names[@]}" -gt 0 ]; then
    printf 'Unconfigured (set PLANNING_CONTEXT_CACHE to run): %s\n' "${unconfigured_names[*]-}"
fi
echo "──────────────────────────────────────────────"

[ "$failed" -eq 0 ]
