#!/usr/bin/env bash
# MODE: DEV
# Work-unit inventory helper contract test (plan-inventory-lib.sh) plus the
# status-label glyphs in plan-document-lib.sh.
#
# The inventory row shape was re-derived by 38 inline `awk -F'|'` programs with
# hard-coded field indices. plan_inventory_* is now the single reader, so its
# TSV field order, its cell normalisation (trim, surrounding backticks, tabs)
# and its not-found signal are the contract every one of those callers depends
# on. plan_status_label's glyphs are the on-disk contract for progress tables.
#
# Usage: test-inventory-helpers.sh
# shellcheck disable=SC2154  # plan_inventory_* are assigned at runtime by the
# sourced plan-inventory-lib row/split helpers

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

scripts="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$scripts/plan-document-lib.sh"

temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-inventory-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

note_fail() { printf 'inventory-helpers: %s\n' "$1" >&2; t_record "$1"; }

assert_eq() {
    local label="$1" want="$2" got="$3"
    [ "$want" = "$got" ] || note_fail "$label: want [$want], got [$got]"
}

# Asserts a command exits with <code>; `!` alone would accept any failure and
# would not notice a crash in place of the expected refusal. The subshell is
# required: plan_die exits, and a bare call would take this test with it.
expect_exit() {
    local want="$1" label="$2" status=0
    shift 2
    ( "$@" ) >/dev/null 2>&1 || status=$?
    [ "$status" -eq "$want" ] || note_fail "$label: want exit $want, got $status"
}

# ── Fixture ──────────────────────────────────────────────────────────────────
# Row W02 carries the awkward cases: leading/trailing padding, backticked cells,
# an interior double space that an unanchored trim would eat, and a literal tab.
inventory="$temporary_root/work-unit-inventory.md"
{
    printf '# Work-unit inventory\n\n'
    printf '## Definition-of-done coverage\n\n'
    printf '| Required outcome or proof | Work unit IDs | Notes |\n|---|---|---|\n'
    printf '| an outcome | W01 | a note |\n\n'
    printf '## Work units\n\n'
    printf '| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |\n'
    printf '|---|---|---|---|---|---|---|---|---|\n'
    printf '| W01 | source | `src/a.php` | `A::run()` | `N/A` | add the guard | — | 01-first | 01-step-alpha |\n'
    printf '|   W02   |  test  |   `src/b.php`   | `B::run()` | N/A | two  spaces and a\ttab | W01, W02 | 01-first | 02-step-beta |\n'
    printf '| W03 | verification | N/A | grader | N/A | grade W01 | W01 | 02-second | 01-step-gamma |\n'
} > "$inventory"

# ── plan_inventory_rows: fixed TSV, one line per work-unit row ───────────────
rows="$(plan_inventory_rows "$inventory")"
assert_eq 'row count' 3 "$(printf '%s\n' "$rows" | wc -l | tr -d ' ')"
assert_eq 'coverage rows are not work-unit rows' 0 "$(printf '%s\n' "$rows" | { grep -c 'an outcome' || true; } | tr -d ' ')"
assert_eq 'W01 TSV' \
    "$(printf 'W01\tsource\tsrc/a.php\tA::run()\tN/A\tadd the guard\t—\t01-first\t01-step-alpha')" \
    "$(printf '%s\n' "$rows" | sed -n 1p)"
assert_eq 'W02 TSV: trimmed, de-backticked, tab folded, interior spaces kept' \
    "$(printf 'W02\ttest\tsrc/b.php\tB::run()\tN/A\ttwo  spaces and a tab\tW01, W02\t01-first\t02-step-beta')" \
    "$(printf '%s\n' "$rows" | sed -n 2p)"

# ── plan_inventory_row / plan_inventory_split: the named value channel ───────
plan_inventory_row "$inventory" W02
assert_eq 'channel id' W02 "$plan_inventory_id"
assert_eq 'channel type' test "$plan_inventory_type"
assert_eq 'channel file' src/b.php "$plan_inventory_file"
assert_eq 'channel scope' 'B::run()' "$plan_inventory_scope"
assert_eq 'channel subscope' N/A "$plan_inventory_subscope"
assert_eq 'channel change' 'two  spaces and a tab' "$plan_inventory_change"
assert_eq 'channel depends' 'W01, W02' "$plan_inventory_depends"
assert_eq 'channel goal' 01-first "$plan_inventory_goal"
assert_eq 'channel step' 02-step-beta "$plan_inventory_step"

plan_inventory_split "$(printf 'W09\tsource\tf\ts\tsub\tc\td\tg\tst')"
assert_eq 'split id' W09 "$plan_inventory_id"
assert_eq 'split step' st "$plan_inventory_step"

# ── A missing id: return 1 and leave no stale cells behind ───────────────────
plan_inventory_row "$inventory" W02
if plan_inventory_row "$inventory" W99; then
    note_fail 'plan_inventory_row returned 0 for an absent id'
fi
assert_eq 'absent id clears the id cell' '' "$plan_inventory_id"
assert_eq 'absent id clears the goal cell' '' "$plan_inventory_goal"
assert_eq 'absent id clears the step cell' '' "$plan_inventory_step"

# ── plan_status_label: the glyphs are the on-disk contract ──────────────────
assert_eq 'label incomplete' '💤 incomplete' "$(plan_status_label incomplete)"
assert_eq 'label in-progress' '⏳ in progress' "$(plan_status_label in-progress)"
assert_eq 'label in_progress' '⏳ in progress' "$(plan_status_label in_progress)"
assert_eq 'label completed' '✅ completed' "$(plan_status_label completed)"
expect_exit 1 'unknown status word' plan_status_label bogus
assert_eq 'unknown status word prints nothing' '' "$(plan_status_label bogus || true)"

[ "$(t_failures)" -eq 0 ] || exit 1
printf '%s\n' 'test-inventory-helpers: PASS'
