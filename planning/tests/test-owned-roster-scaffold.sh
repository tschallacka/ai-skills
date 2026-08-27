#!/usr/bin/env bash
# MODE: DEV
# B21 regression — an emptied Owned-work-units roster returns to the created
# shape instead of disappearing.
#
# plan_rewrite_owned_work_units emitted one §9.N paragraph per remaining
# inventory row and nothing at all when none remained, so removing a goal's
# last work unit deleted the whole numbered roster. The next add-work-unit.sh
# then refused ("no numbered Owned work units section"), which is the
# precondition that turned B20 into a stranded half-created unit on the VPS.
#
# Now a roster with zero rows emits the scaffold add-goal.sh creates: a §9.1
# holding '<add work units with add-work-unit.sh>', which the placeholder
# registry covers and which add-work-unit.sh replaces directly.
#
# Control: removing one of several units keeps the survivors renumbered from
# §9.1 with their change text intact.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-roster-scaffold-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

fail() { t_fail "$*"; }

plan="$temporary_root/roster"
"$script_dir/create-plan.sh" "$plan" scaffold >/dev/null
"$script_dir/add-goal.sh" "$plan" 01-g 'G' 'an outcome' >/dev/null
"$script_dir/add-work-unit.sh" "$plan" --id W01 --type source --file a.php \
    --scope 'A::x' --subscope N/A --change 'change A' \
    --depends-on '—' --goal 01-g --step 01-step-a >/dev/null

# --- the B21 shape: removing the only unit leaves the scaffold ---------------
rc=0
"$script_dir/remove-work-unit.sh" "$plan" W01 --confirm-cascade >/dev/null 2>&1 || rc=$?
t_assert_eq "remove of the last unit exits 0" "$rc" 0
goal="$plan/01-g/goal.md"
t_assert_eq "the §9.1 label survives an emptied roster" \
    "$(grep -c '^§ 9\.1$' "$goal" || true)" 1
t_assert_eq "the add-scaffold placeholder survives an emptied roster" \
    "$(grep -cF '<add work units with add-work-unit.sh>' "$goal" || true)" 1

# --- and the next add takes the normal path again -----------------------------
rc=0
"$script_dir/add-work-unit.sh" "$plan" --id W02 --type docs --file b.md \
    --scope 'B' --subscope N/A --change 'change B' \
    --depends-on '—' --goal 01-g --step 02-step-b >/dev/null 2>&1 || rc=$?
t_assert_eq "re-adding after an emptied roster exits 0" "$rc" 0
t_assert_eq "inventory row present after re-add" \
    "$(grep -cF '| W02 |' "$plan/work-unit-inventory.md" || true)" 1
grep -qF '`W02` — change B' "$goal" || fail "roster entry missing after re-add"

# --- control: survivors keep their labels and text ----------------------------
plan_many="$temporary_root/many"
"$script_dir/create-plan.sh" "$plan_many" many >/dev/null
"$script_dir/add-goal.sh" "$plan_many" 01-g 'G' 'an outcome' >/dev/null
"$script_dir/add-work-unit.sh" "$plan_many" --id W01 --type source --file a.php \
    --scope 'A::x' --subscope N/A --change 'change A' \
    --depends-on '—' --goal 01-g --step 01-step-a >/dev/null
"$script_dir/add-work-unit.sh" "$plan_many" --id W02 --type source --file b.php \
    --scope 'B::x' --subscope N/A --change 'change B' \
    --depends-on '—' --goal 01-g --step 02-step-b >/dev/null
"$script_dir/remove-work-unit.sh" "$plan_many" W01 --confirm-cascade >/dev/null 2>&1
many_goal="$plan_many/01-g/goal.md"
t_assert_eq "surviving unit is relabelled §9.1" \
    "$(grep -c '^§ 9\.1$' "$many_goal" || true)" 1
grep -qF '`W02` — change B' "$many_goal" || fail "survivor lost its roster line"
t_assert_eq "no stray placeholder alongside a survivor" \
    "$(grep -cF '<add work units with add-work-unit.sh>' "$many_goal" || true)" 0

t_end
