#!/usr/bin/env bash
# MODE: DEV
# B20 regression — add-work-unit.sh stages all three writes before any lands.
#
# The inventory row and step file used to be renamed into place before the
# goal-roster edit ran, so a goal whose "Owned work units" section held no
# numbered paragraphs died mid-call with the row and step already committed:
# the retry then hit "Work-unit ID already exists" and the unit was stranded
# half-created (row + step exist, roster missing). The docblock promises the
# opposite: "All three land together or none does."
#
# Now the goal edit is staged while the other two are still temps, so a
# roster failure leaves the plan exactly as it was.
#
# Controls: a healthy goal (placeholder roster) still gets all three writes,
# and the refusal message is unchanged.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-wu-staging-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

fail() { t_fail "$*"; }

strip_roster() { # <goal-file> — drop every §9.x label and paragraph, keeping headings
    awk '
        in_roster && /^## / { in_roster = 0 }
        in_roster { next }
        /^## Owned work units$/ { print; in_roster = 1; next }
        { print }
    ' "$1"
}

seed_plan() { # <plan-dir>
    local plan="$1"
    "$script_dir/create-plan.sh" "$plan" staging >/dev/null
    "$script_dir/add-goal.sh" "$plan" 01-g 'G' 'an outcome' >/dev/null
}

# --- the B20 shape: roster gone, add must refuse WITHOUT stranding ----------
plan_broken="$temporary_root/broken"
seed_plan "$plan_broken"
"$script_dir/add-work-unit.sh" "$plan_broken" W01 source a.php 'A::x' N/A 'change A' '—' 01-g 01-step-a >/dev/null
strip_roster "$plan_broken/01-g/goal.md" > "$plan_broken/01-g/goal.md.new"
mv "$plan_broken/01-g/goal.md.new" "$plan_broken/01-g/goal.md"

rc=0
out="$(mktemp "$temporary_root/refusal.XXXXXX")"
"$script_dir/add-work-unit.sh" "$plan_broken" W02 docs b.md 'B' N/A 'change B' '—' 01-g 02-step-b >/dev/null 2>"$out" || rc=$?
t_assert_eq "the call still refuses on an unusable roster" "$([ "$rc" -ne 0 ] && echo yes || echo no)" yes
grep -q 'no numbered Owned work units section' "$out" || fail "refusal message changed: $(cat "$out")"
t_assert_eq "no inventory row was left behind" \
    "$(grep -cF '| W02 |' "$plan_broken/work-unit-inventory.md" || true)" 0
if [ -e "$plan_broken/01-g/steps/02-step-b.md" ]; then
    fail "the step file was left behind by the refused call"
fi

# --- control: a healthy goal gets all three writes ---------------------------
plan_ok="$temporary_root/ok"
seed_plan "$plan_ok"
rc=0
"$script_dir/add-work-unit.sh" "$plan_ok" W03 source c.php 'C::x' N/A 'change C' '—' 01-g 03-step-c >/dev/null 2>&1 || rc=$?
t_assert_eq "healthy-goal add exits 0" "$rc" 0
t_assert_eq "inventory row present" "$(grep -cF '| W03 |' "$plan_ok/work-unit-inventory.md" || true)" 1
if [ ! -f "$plan_ok/01-g/steps/03-step-c.md" ]; then
    fail "step file missing on success"
fi
grep -qF '`W03` — change C' "$plan_ok/01-g/goal.md" || fail "roster entry missing on success"

t_end
