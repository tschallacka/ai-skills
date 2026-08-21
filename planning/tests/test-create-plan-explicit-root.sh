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

# ---- a refused call leaves nothing on disk -------------------------------
# The parent is created for an explicit path, so it has to be created after the
# guards rather than while parsing the arguments: contract 2 puts the
# irreversible step last. It ran first, so a bad plan name created the parent and
# then died.
refused_root="$work/refused"
rc=0
"$scripts/create-plan.sh" "$refused_root/BadName" 'A title' >/dev/null 2>&1 || rc=$?
t_assert_eq "a non-kebab plan name is refused" "$rc" 64
[ ! -d "$refused_root" ] \
    || t_fail "a refused call created its parent directory: $refused_root"

mkdir -p "$work/occupied/taken-name"
rc=0
"$scripts/create-plan.sh" "$work/occupied/taken-name" 'A title' >/dev/null 2>&1 || rc=$?
t_assert_eq "an existing plan directory is refused" "$rc" 73

t_end "test-create-plan-explicit-root"
