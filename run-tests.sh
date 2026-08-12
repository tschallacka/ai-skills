#!/usr/bin/env bash
# Deterministic, repeatable test runner for the whole repo.
#
# Runs every test under planning/tests/ and benchmark/planning/tests/ in a
# fixed sorted order, each under the resource-limited wrapper, and reports a
# stable summary. The order and per-test result are the same on every run on
# the same host, so CI or a maintainer sees identical output.
#
# Usage: run-tests.sh [--verbose]
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
wrapper="$repo_root/resource-limited-testing/scripts/limited-run.sh"
verbose=false
[ "${1:-}" = "--verbose" ] && verbose=true

# Tests that require PLANNING_CONTEXT_CACHE (a developer-only legacy context
# cache). They are documented to fail closed when the fixture is absent; the
# runner prefers to run them when the fixture is configured and otherwise
# reports them as UNCONFIGURED rather than as a silent skip or a hard failure.
context_gated=(
    planning/tests/test-plan-context.sh
    planning/tests/test-plan-context-deferred-boundary.sh
)

is_context_gated() {
    local t="$1" entry
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

suites=(
    planning/tests
    benchmark/planning/tests
)

tests=()
for suite in "${suites[@]}"; do
    while IFS= read -r t; do
        tests+=("$t")
    done < <(discover "$suite")
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
    if [ -d "${suite}/" ]; then :; fi
    # benchmark tests spin up worker/reviewer-like processes; give them headroom.
    if [[ "$t" == benchmark/* ]]; then mem=6G; cpu=400; else mem=2G; cpu=400; fi

    if is_context_gated "$t" && [ -z "${PLANNING_CONTEXT_CACHE:-}" ]; then
        unconfigured=$((unconfigured + 1))
        unconfigured_names+=("$label")
        printf '  %-52s UNCONFIGURED (PLANNING_CONTEXT_CACHE)\n' "$label"
        return
    fi

    total=$((total + 1))
    if "$wrapper" "$mem" "$cpu" -- bash "$t" >/tmp/run-tests.$$.out 2>&1; then
        passed=$((passed + 1))
        printf '  %-52s PASS\n' "$label"
    else
        code=$?
        failed=$((failed + 1))
        failed_names+=("$label")
        printf '  %-52s FAIL (exit %s)\n' "$label" "$code"
        if [ "$verbose" = true ]; then
            sed 's/^/      /' /tmp/run-tests.$$.out | tail -20
        fi
    fi
    rm -f /tmp/run-tests.$$.out
}

start="$(date -u +%s)"
echo "Testing $(basename "$repo_root") — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Runner order: sorted test files under planning/tests then benchmark/planning/tests"
echo

for t in "${tests[@]}"; do
    run_one "$t"
done

elapsed="$(( $(date -u +%s) - start ))"
echo
echo "──────────────────────────────────────────────"
printf 'Total ran: %d   Passed: %d   Failed: %d   Unconfigured: %d\n' "$total" "$passed" "$failed" "$unconfigured"
printf 'Elapsed: %ds\n' "$elapsed"
if [ "${#failed_names[@]}" -gt 0 ]; then
    printf 'Failed: %s\n' "${failed_names[*]}"
fi
if [ "${#unconfigured_names[@]}" -gt 0 ]; then
    printf 'Unconfigured (set PLANNING_CONTEXT_CACHE to run): %s\n' "${unconfigured_names[*]}"
fi
echo "──────────────────────────────────────────────"

[ "$failed" -eq 0 ]