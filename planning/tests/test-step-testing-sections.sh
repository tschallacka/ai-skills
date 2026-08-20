#!/usr/bin/env bash
# test-step-testing-sections.sh — a testing companion can carry every
# verification section the section forms advertise.
#
# `update-plan-content.sh -ss ... browser-verification` accepted the id while no
# helper could create the section, so it failed with a missing heading on every
# companion. An external report inferred that re-creating with --overwrite would
# add it; that was verified false — the creator emitted "## Automated tests" and
# nothing else. --browser, --backend and --manual close that gap.
#
# Section numbers are fixed per section name, not by position: a companion
# holding only automated tests and manual verification still labels the manual
# paragraphs § 5.x, so -ss and -sp address the same labels whatever exists.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts_dir="$repo_root/planning/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/step-testing-sections.XXXXXX")"
trap 'rm -rf "$work"' EXIT
plan="$work/plan"
cp -R "$repo_root/benchmark/planning/tests/fixtures/self-hosted-plan" "$plan"
goal_dir="$plan/01-plan-dir-synonym"
step='01-step-confirm-hoister'
companion="$goal_dir/steps/${step}-testing.md"

# ---- every section, with its own fixed number --------------------------------
"$scripts_dir/create-step-testing.sh" "$goal_dir" "$step" \
    'Run the suite on both shells.' \
    --browser 'Load the page.\nConfirm the button renders.' \
    --backend 'Check the API returns 200.' \
    --manual 'Confirm with a screen reader.' \
    --overwrite >/dev/null \
    || t_fail 'the creator refused a companion carrying every verification section'

for heading in '## Automated tests' '## Browser verification' '## Backend verification' '## Manual verification'; do
    grep -Fqx "$heading" "$companion" || t_fail "the companion is missing $heading"
done
grep -Fqx '§ 2.1' "$companion" || t_fail 'automated tests are not numbered 2.x'
grep -Fqx '§ 3.1' "$companion" || t_fail 'browser verification is not numbered 3.x'
grep -Fqx '§ 3.2' "$companion" || t_fail 'the second browser paragraph did not get its own label'
grep -Fqx '§ 4.1' "$companion" || t_fail 'backend verification is not numbered 4.x'
grep -Fqx '§ 5.1' "$companion" || t_fail 'manual verification is not numbered 5.x'

# ---- a section form can now address what the creator emitted -----------------
rc=0
"$scripts_dir/update-plan-content.sh" -ss "$plan" "01-plan-dir-synonym/${step}-testing" \
    browser-verification -p '3.1: a rewritten browser check.' >/dev/null 2>&1 || rc=$?
t_assert_eq 'a section form can rewrite browser verification' "$rc" 0
grep -Fq 'a rewritten browser check.' "$companion" \
    || t_fail 'the rewritten browser paragraph is not in the companion'

# ---- numbering is per name, not per position --------------------------------
"$scripts_dir/create-step-testing.sh" "$goal_dir" "$step" \
    'Run the suite.' --manual 'Ask a human.' --overwrite >/dev/null \
    || t_fail 'the creator refused a companion with automated and manual sections only'
grep -Fqx '§ 5.1' "$companion" \
    || t_fail 'manual verification lost its fixed number when earlier sections were absent'
grep -Fqx '## Browser verification' "$companion" \
    && t_fail 'a section that was not supplied was emitted anyway'

# ---- a rejected section leaves the companion alone --------------------------
# The docblock promises validation before any filesystem change, and that has to
# hold for the flags too, or a bad --manual truncates a good companion.
before="$(cksum < "$companion")"
rc=0
"$scripts_dir/create-step-testing.sh" "$goal_dir" "$step" 'Run the suite.' \
    --manual 'a paragraph with a § marker in it' --overwrite >/dev/null 2>&1 || rc=$?
[ "$rc" -ne 0 ] || t_fail 'the creator accepted a manual section containing the reserved marker'
t_assert_eq 'the companion is untouched after a rejected section' \
    "$(cksum < "$companion")" "$before"

t_end
