#!/usr/bin/env bash
# MODE: DEV
# test-create-plan-explicit-root.sh — explicit plan paths own their root.
#
# A slash-bearing create-plan.sh argument is already a full destination. It must
# not inspect HOME/.plans or any unrelated global root while deciding whether a
# new plan may be created.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts="$repo_root/planning/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/create-plan-explicit.XXXXXX")"
trap 'rm -rf "$work"' EXIT

home="$work/home"
target_root="$work/target-root"
mkdir -p "$home/.plans" "$target_root"

old_plan="$home/.plans/old/01-goal/steps"
mkdir -p "$old_plan"
printf '# Step\n' > "$old_plan/01-step-a.md"
printf '# Step\n' > "$old_plan/01-step-b.md"

rc=0
HOME="$home" "$scripts/create-plan.sh" "$target_root/explicit-plan" \
    "Explicit root probe" > "$work/create.out" 2> "$work/create.err" || rc=$?

t_assert_eq "explicit path create-plan exits 0" "$rc" 0
[ -f "$target_root/explicit-plan/plan-description.md" ] \
    || t_fail "explicit plan-description.md was not created"
[ -f "$target_root/.env" ] \
    || t_fail "explicit parent did not receive root env metadata"
if grep -Fq "old" "$work/create.err"; then
    t_fail "explicit create-plan inspected the unrelated HOME/.plans root"
fi
if grep -Fq "REFUSING TO CREATE A PLAN" "$work/create.err"; then
    t_fail "explicit create-plan refused because of unrelated global duplicates"
fi

t_end "test-create-plan-explicit-root"
