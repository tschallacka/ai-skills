#!/usr/bin/env bash
# MODE: DEV
# test-validation-readiness-summary.sh — plain validation may not call a plan
# passed while its Instructions are placeholders.
#
# A real run printed "Plan validation passed: 36 work units" with 138
# placeholders still in place, every step's Instructions reading
# `<direct action on this one target>`. A registered placeholder is deliberately
# a WARN and not an error -- promoting it would break the create-then-fill flow --
# so what changed is the summary, which is the line a reader takes as the
# readiness verdict.
#
# The exit code stays 0 in both cases, and that is asserted, because the fix must
# not smuggle in a gate that common-lib contract 1 forbids.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts="$repo_root/planning/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/validation-readiness.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# A plan that validates clean structurally: goal, two units, coverage for both,
# a testing requirement, testing companions, a review, and a ticked
# decomposition list. Placeholders are left exactly as the writers emit them.
build_plan() {
    local root="$1" plan
    mkdir -p "$root"
    PLANS_ROOT="$root" "$scripts/create-plan.sh" demo 'A fixture plan' >/dev/null
    plan="$root/demo"
    "$scripts/add-goal.sh" "$plan" 01-first 'First goal' \
        'the renderer emits stable output' >/dev/null
    "$scripts/add-work-unit.sh" "$plan" --id W01 --type source --file src/a.c \
        --scope sym_a --subscope N/A --change 'change a' --depends-on '—' \
        --goal 01-first --step 01-step-do-it >/dev/null
    "$scripts/add-work-unit.sh" "$plan" --id W02 --type verification --file N/A \
        --scope N/A --subscope N/A --change 'prove the output' --depends-on W01 \
        --goal 01-first --step 02-step-prove >/dev/null
    "$scripts/add-coverage.sh" "$plan" 'the renderer emits stable output' W01 'the source unit' >/dev/null
    "$scripts/add-coverage.sh" "$plan" 'the output is proven by a test' W02 'the verification unit' >/dev/null
    "$scripts/update-plan-content.sh" -tr "$plan" 01-first yes 'the output is observable' >/dev/null
    "$scripts/create-step-testing.sh" "$plan/01-first" 01-step-do-it 'run it and diff' >/dev/null
    "$scripts/create-step-testing.sh" "$plan/01-first" 02-step-prove 'run it and diff' >/dev/null
    "$scripts/create-adversarial-review.sh" "$plan" >/dev/null 2>&1
    t_sed_i 's/^- \[ \]/- [x]/' "$plan/work-unit-inventory.md"
    printf '%s\n' "$plan"
}

plan="$(build_plan "$work/withph")"

# ── a structurally sound plan full of placeholders is not "passed" ──────────
out="$("$scripts/validate-plan.sh" "$plan" 2>&1)" || t_fail 'validation of a sound plan failed'
rc=0
"$scripts/validate-plan.sh" "$plan" >/dev/null 2>&1 || rc=$?
t_assert_eq 'placeholders do not turn plain validation into a failure' "$rc" '0'
t_assert_eq 'and the summary does not say passed' \
    "$(printf '%s\n' "$out" | grep -c 'validation passed' || true)" '0'
t_assert_contains 'it says incomplete' 'Plan validation incomplete' "$out"
# The count is the point: "some placeholders" is not actionable, a number is.
t_assert_eq 'with a placeholder count above zero' \
    "$(printf '%s\n' "$out" | sed -n 's/.*goals, \([0-9]*\) placeholder.*/\1/p' \
        | awk '$1 > 0 { print "counted" }')" 'counted'
t_assert_contains 'and a remedy rather than just a verdict' 'fill the placeholders' "$out"
# The structural work still has to be reported, or the summary trades one
# misleading line for another.
t_assert_contains 'the work units are still counted' '2 work units across 1 goals' "$out"

# ── filling them restores the word ─────────────────────────────────────────
filled="$(build_plan "$work/filled")"
while IFS= read -r doc; do
    t_sed_i 's/<[^<>]*[A-Za-z][^<>]*>/real authored prose/g' "$doc"
done < <(find "$filled" -name '*.md')
out_filled="$("$scripts/validate-plan.sh" "$filled" 2>&1)" || t_fail 'validation of the filled plan failed'
t_assert_contains 'a filled plan is passed' 'Plan validation passed' "$out_filled"
t_assert_eq 'and is not called incomplete' \
    "$(printf '%s\n' "$out_filled" | grep -c 'incomplete' || true)" '0'
# An unrelated advisory must not suppress the word: only placeholders do.
t_assert_contains 'even with the mid-cycle review warning present' \
    'Adversarial review is not approved' "$out_filled"

# ── --complete still promotes them, which is the flow the WARN exists for ───
rc=0
"$scripts/validate-plan.sh" --complete "$plan" >/dev/null 2>&1 || rc=$?
t_assert_eq 'under --complete a placeholder is still an error' \
    "$([ "$rc" -ne 0 ] && printf refused)" 'refused'

t_end
