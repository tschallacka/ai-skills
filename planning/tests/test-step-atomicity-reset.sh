#!/usr/bin/env bash
# MODE: DEV
# test-step-atomicity-reset — reverting a step to incomplete clears its
# atomicity boxes, and the completion path still ticks them.
#
# Usage: test-step-atomicity-reset.sh
#
# A ticked box asserts that a mechanical check compared the unit's declared
# target against the git diff of its completion. When the step goes back to
# incomplete that diff no longer stands, so the assertion must not survive it
# (B64). The inverse contract is asserted here too: this must not become a
# script that unticks and never ticks.
#
# This file is shipped, so it holds to the shipped-runtime dependency rule in
# CODE-STYLE.md §1: bash, POSIX coreutils, awk, sed, grep, rjq only. No python3.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
scripts="$root/planning/scripts"

note_fail() { printf 'step-atomicity-reset: %s\n' "$1" >&2; t_record "$1"; }

work="$(mktemp -d "${TMPDIR:-/tmp}/atomicity-reset.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# How many of the three boxes in a step file are ticked.
ticked() {
    awk '/^- \[[xX]\] (This step owns|No other file|Any follow-on)/ { n++ } END { print n + 0 }' "$1"
}

export PLANS_ROOT="$work/plans"
mkdir -p "$PLANS_ROOT"
plan="$PLANS_ROOT/reset-fixture"
"$scripts/create-plan.sh" reset-fixture "A fixture plan" >/dev/null
"$scripts/add-goal.sh" "$plan" 01-first-goal "First goal" \
    "the step tracker and the step file agree" >/dev/null
"$scripts/add-work-unit.sh" "$plan" --id W01 --type source \
    --file "src/only.rs" --scope "only()" --subscope N/A \
    --change "The one target this fixture declares." --depends-on -- \
    --goal 01-first-goal --step 01-step-only >/dev/null

goal_dir="$plan/01-first-goal"
step_file="$goal_dir/steps/01-step-only.md"

# 1. A freshly added step starts unticked: the check has not run.
[ "$(ticked "$step_file")" -eq 0 ] || note_fail "a new step starts with ticked boxes"

# 2. Tick them the way a completion does, then revert and require them cleared.
#    Ticking by hand rather than through a git-backed completion keeps the
#    fixture free of a repository, while asserting exactly the state that
#    reverting must undo.
sed -e 's/^- \[ \] This step owns/- [x] This step owns/' \
    -e 's/^- \[ \] No other file/- [x] No other file/' \
    -e 's/^- \[ \] \(Any follow-on.*\)$/- [x] \1 VIOLATION: also touched src\/other.rs/' \
    "$step_file" > "$step_file.t" && mv "$step_file.t" "$step_file"
[ "$(ticked "$step_file")" -eq 3 ] || note_fail "fixture setup did not tick all three boxes"

"$scripts/update-step.sh" "$goal_dir" 01-step-only incomplete >/dev/null 2>&1
[ "$(ticked "$step_file")" -eq 0 ] \
    || note_fail "reverting to incomplete left $(ticked "$step_file") box(es) ticked"
if awk '/VIOLATION/ { found = 1 } END { exit !found }' "$step_file"; then
    note_fail "reverting to incomplete left the VIOLATION annotation behind"
fi

# 3. in-progress is not a revert: it must leave the boxes alone, so a step
#    being worked on does not lose evidence it already earned.
sed -e 's/^- \[ \] This step owns/- [x] This step owns/' "$step_file" > "$step_file.t" \
    && mv "$step_file.t" "$step_file"
"$scripts/update-step.sh" "$goal_dir" 01-step-only in-progress >/dev/null 2>&1
[ "$(ticked "$step_file")" -eq 1 ] \
    || note_fail "in-progress changed the boxes, which only completion and revert may do"

# 4. The completion path still ticks: without this the reset could be
#    implemented by never ticking at all, and every assertion above would pass.
"$scripts/update-step.sh" "$goal_dir" 01-step-only incomplete >/dev/null 2>&1
repo="$work/repo"
mkdir -p "$repo/src"
git -C "$repo" init -q 2>/dev/null || { printf 'step-atomicity-reset: SKIP (no git)\n'; exit 0; }
git -C "$repo" config user.email t@example.com
git -C "$repo" config user.name t
printf 'seed\n' > "$repo/src/only.rs"
git -C "$repo" add -A && git -C "$repo" commit -qm seed
printf 'changed\n' > "$repo/src/only.rs"
"$scripts/update-step.sh" "$goal_dir" 01-step-only completed \
    --repo-root "$repo" --unit W01 >/dev/null 2>&1
[ "$(ticked "$step_file")" -eq 3 ] \
    || note_fail "completion with a matching diff ticked $(ticked "$step_file") of 3 boxes"

[ "$(t_failures)" -eq 0 ] || exit 1
printf 'step-atomicity-reset: PASS\n'
