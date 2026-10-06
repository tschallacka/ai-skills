#!/usr/bin/env bash
# MODE: DEV
# test-reopen-retires-fix-claims.sh — reopening a plan retires its fix claims (B397).
#
# A plan approved, then reopened, must not keep the claims recorded for that
# approval in fixes.md: the next review cycle mints under a new session, so the
# old claims fail verification and block a fresh claim for the same pair. Moving
# fixes.md aside on the reopen keeps what was approved and lets the next approval
# ask for a complete new set.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts_dir="$repo_root/planning/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/reopen-retires.XXXXXX")"
trap 'rm -rf "$work"' EXIT

plan="$work/plan"
"$scripts_dir/create-plan.sh" "$plan" reopen >/dev/null

# A reviewed plan with claims on disk from its last approval.
"$scripts_dir/create-adversarial-review.sh" "$plan" >/dev/null
printf 'AR-01\tW01\t%s\n' 'a1b2c3' > "$plan/fixes.md"

if ! "$scripts_dir/update-plan-content.sh" --review-status "$plan" pending >/dev/null 2>&1; then
    t_fail "reopening the plan (--review-status pending) was refused"
fi

if [ -e "$plan/fixes.md" ]; then
    t_fail "fixes.md still holds the claims from the last approval after a reopen"
fi
retired="$(find "$plan" -maxdepth 1 -name 'fixes.superseded-*.md' | head -n 1)"
if [ -z "$retired" ]; then
    t_fail "the retired claims were not kept beside the plan"
elif ! grep -q 'AR-01' "$retired"; then
    t_fail "the retired claims lost their content"
fi

t_end
