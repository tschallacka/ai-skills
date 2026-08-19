#!/usr/bin/env bash
# test-self-hosted-plan.sh — a plan authored entirely through this skill's own
# helpers still validates.
#
# The fixture under benchmark/planning/tests/fixtures/self-hosted-plan was
# produced by driving create-plan.sh, add-goal.sh, add-work-unit.sh,
# add-coverage.sh, update-plan-content.sh and create-step-testing.sh -- no file
# was hand-written. That makes it the one fixture that fails if the helpers and
# the gate ever disagree about what a complete plan looks like, which no
# hand-built fixture can catch.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts_dir="$repo_root/planning/scripts"
fixture="$repo_root/benchmark/planning/tests/fixtures/self-hosted-plan"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/self-hosted-plan.XXXXXX")"
trap 'rm -rf "$work"' EXIT

[ -d "$fixture" ] || t_fail 'the self-hosted plan fixture is missing'
plan="$work/plan"
cp -R "$fixture" "$plan"

t_expect_exit 0 'the self-hosted plan validates' "$scripts_dir/validate-plan.sh" "$plan"
t_expect_exit 0 'it validates with propagation' \
    "$scripts_dir/validate-plan.sh" --propagation "$plan"
t_expect_exit 0 'it validates via the --plan-dir synonym' \
    "$scripts_dir/validate-plan.sh" --plan-dir "$plan"
# The wording sweep is advisory, so it must not change the verdict.
t_expect_exit 0 'the advisory wording sweep does not gate it' \
    "$scripts_dir/validate-plan.sh" --stale default "$plan"

# Both readers must serve it, since a reviewer reads a plan through them.
for id in plan inventory coverage adversarial-review; do
    t_expect_exit 0 "plan-content.sh serves '$id'" \
        "$scripts_dir/plan-content.sh" get "$plan" "$id"
done
out="$(PLANNING_CONTEXT_CACHE=1 "$scripts_dir/plan-context.sh" read \
    --plan-dir "$plan" --document inventory --read-only 2>&1 || true)"
t_assert_contains 'the bounded reader serves its inventory' '| W01 |' "$out"

# What makes this fixture load-bearing: it was produced by the helpers, so a
# helper that stops emitting a surface the gate requires breaks it here.
for required in plan-description.md work-unit-inventory.md adversarial-review.md progress.md; do
    [ -f "$plan/$required" ] || t_fail "the authored plan has no $required"
done
units="$({ grep -c '^| W' "$plan/work-unit-inventory.md" || true; })"
t_assert_eq 'every authored work unit is present' "$units" 8
companions="$(find "$plan" -name '*-testing.md' | { grep -c . || true; })"
t_assert_eq 'every step has its testing companion' "$companions" 8

t_end
