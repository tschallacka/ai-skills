#!/usr/bin/env bash
# MODE: DEV
# test-ui-prohibition-scope.sh — the run-cache prohibition applies to the input.
#
# The rule is right: a UI story must be driven by a real interaction, not by
# console or state manipulation. It was applied to the whole file, so the word
# localStorage in a starting state or a result failed too. A reporter reworded
# truthful evidence to get past it, which is the worst thing a gate can do -- it
# bought compliance by making the artifact less accurate.
#
# validate_story_cache is called directly with the globals it reads, rather than
# through a whole UI-validated plan: the unit under test is which part of the
# file the pattern is matched against.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts="$repo_root/planning/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/ui-prohibition.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# A cache with a prohibited word in each named section, and a clean interaction
# row, so one file exercises every placement.
write_cache() { # <interaction-row>
    mkdir -p "$work/plan/ui-story-runs"
    {
        printf '# Browser run cache: US-01\n\n'
        printf '## Starting state\n\n'
        printf 'The page keeps the chosen filter in localStorage, so the run starts with it cleared.\n\n'
        printf '## Buffered interaction sequence\n\n'
        printf '| Order | Direct UI input | Target / value | Expected readiness signal |\n'
        printf '|---|---|---|---|\n'
        printf '%s\n\n' "$1"
        printf '## Waits and readiness\n\n'
        printf 'Wait for the list to repaint.\n\n'
        printf '## Run result\n\n'
        printf 'The filter survived a reload, which is what localStorage is for.\n'
    } > "$work/plan/ui-story-runs/US-01.md"
}

# errors is the counter fail() increments; the rest are globals the function reads.
run_validate() { # → the number of findings
    plan_dir="$work/plan" complete_mode=false errors=0
    # shellcheck source=planning/scripts/validate-plan-common-lib.sh
    source "$scripts/validate-plan-common-lib.sh"
    # shellcheck source=planning/scripts/validate-plan-ui-lib.sh
    source "$scripts/validate-plan-ui-lib.sh"
    validate_story_cache US-01 ui-story-runs/US-01.md '💤 untested' 2>/dev/null
    printf '%s' "$errors"
}
findings() { "$BASH" -c "work='$work'; scripts='$scripts'; $(declare -f run_validate); run_validate"; }

# ── honest prose outside the interaction sequence passes ────────────────────
write_cache '| 1 | click | the Apply button | the results list repaints |'
t_assert_eq 'localStorage in a starting state and a result is not an input' "$(findings)" '0'

# ── the same word inside the interaction row fails ──────────────────────────
write_cache '| 1 | set localStorage directly | the filter key | the list repaints |'
t_assert_eq 'localStorage as the input is refused' \
    "$([ "$(findings)" -gt 0 ] && printf refused)" 'refused'
write_cache '| 1 | evaluate( document.querySelector ) | the button | the list repaints |'
t_assert_eq 'an evaluate( call as the input is refused' \
    "$([ "$(findings)" -gt 0 ] && printf refused)" 'refused'

# ── and an interaction is still required, in that section ───────────────────
write_cache '| 1 | reload the page | the whole document | the list repaints |'
t_assert_eq 'a row with no direct interaction word is refused' \
    "$([ "$(findings)" -gt 0 ] && printf refused)" 'refused'
# The interaction word has to be in the sequence, not merely somewhere in the
# file: prose in a result is not evidence that anything was clicked.
write_cache '| 1 | reload the page | the whole document | click here later |'
t_assert_eq 'a readiness column mentioning click does count, being in the section' \
    "$(findings)" '0'

t_end
