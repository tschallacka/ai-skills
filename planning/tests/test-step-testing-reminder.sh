#!/usr/bin/env bash
# MODE: DEV
# test-step-testing-reminder — the companion reminder fires only when the
# companion was already behind the step, never on every edit.
#
# Usage: test-step-testing-reminder.sh
#
# The reminder used to fire whenever a companion existed. That made it true on
# every step edit, so it could not mark the edit that left a companion behind --
# the one case it exists for (T67). Its value is in the runs where it is silent,
# so silence is asserted here as strictly as the firing is.
#
# This file is shipped, so it holds to the shipped-runtime dependency rule in
# CODE-STYLE.md §1: bash, POSIX coreutils, awk, sed, grep, jq only. No python3.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
scripts="$root/planning/scripts"

note_fail() { printf 'step-testing-reminder: %s\n' "$1" >&2; t_record "$1"; }

work="$(mktemp -d "${TMPDIR:-/tmp}/reminder.XXXXXX")"
trap 'rm -rf "$work"' EXIT

export PLANS_ROOT="$work"
plan="$work/p"
"$scripts/create-plan.sh" p "Reminder fixture" >/dev/null
"$scripts/add-goal.sh" "$plan" 01-a "A" "one demonstrable outcome" >/dev/null
"$scripts/add-work-unit.sh" "$plan" --id W01 --type source --file src/a.rs \
    --scope "a()" --subscope N/A --change "The one target." --depends-on -- \
    --goal 01-a --step 01-step-a >/dev/null
"$scripts/update-plan-content.sh" -tr "$plan" 01-a yes "testing is required here" >/dev/null

# edit_step N TEXT -> the reminder text on stderr, empty when silent
edit_step() {
    "$scripts/update-plan-content.sh" -sp "$plan" 01-a/01-step-a "$1" "$2" >/dev/null 2>&1
}

# 1. No companion: the reminder names the gap, whatever the timestamps say.
case "$(edit_step 5.1 first)" in
    *'continue with its test/proof step'*) ;;
    *) note_fail "with no companion, the missing-companion reminder did not fire" ;;
esac

"$scripts/create-step-testing.sh" "$plan/01-a" 01-step-a "verify it" >/dev/null
sleep 2

# 2. Companion current: silent. This is the assertion that fails if the
#    reminder ever goes back to firing on mere existence.
out="$(edit_step 6.1 second)"
[ -z "$out" ] || note_fail "companion was current but the reminder fired: $out"

sleep 2

# 3. Step 2 left the companion behind; the next edit must say so.
out="$(edit_step 7.1 third)"
case "$out" in
    *'was already behind'*) ;;
    *) note_fail "companion was behind but the reminder was silent" ;;
esac

# 4. Refresh the companion and the reminder goes quiet again, so a reader who
#    acts on it is not told twice for one lapse.
"$scripts/create-step-testing.sh" "$plan/01-a" 01-step-a "verify it again" --overwrite >/dev/null
sleep 2
out="$(edit_step 5.1 fourth)"
[ -z "$out" ] || note_fail "companion was refreshed but the reminder still fired: $out"

[ "$(t_failures)" -eq 0 ] || exit 1
printf 'step-testing-reminder: PASS\n'
