#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# test-goal-testing-row — the testing-requirement row survives every mutation,
# and a yes/no table outside its registered section is reported.
#
# Usage: test-goal-testing-row.sh
#
# Two contracts, one fixture family:
#   1. plan_rewrite_owned_work_units (add/remove work unit) preserves the row
#      from '## Testing requirement' only. An earlier yes/no table elsewhere in
#      the goal — a hand-written '| Blocking | Resolved |', say — used to win
#      the scan and overwrite the author's rationale silently.
#   2. validate-plan fails a goal.md carrying a yes/no first-column table under
#      a heading planning/goal-tables.json does not register.
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

note_fail() { printf 'goal-testing-row: %s\n' "$1" >&2; t_record "$1"; }

assert_eq() {
    local expected="$1" actual="$2" what="$3"
    [ "$expected" = "$actual" ] || note_fail "$what: expected '$expected', got '$actual'"
}

assert_contains() {
    local haystack="$1" needle="$2" what="$3"
    case "$haystack" in
        *"$needle"*) ;;
        *) note_fail "$what: '$needle' not found in output" ;;
    esac
}

assert_not_contains() {
    local haystack="$1" needle="$2" what="$3"
    case "$haystack" in
        *"$needle"*) note_fail "$what: '$needle' unexpectedly present in output" ;;
    esac
}

work="$(mktemp -d "${TMPDIR:-/tmp}/goal-testing-row.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# The row currently in the '## Testing requirement' section, scoped the same way
# the helpers scope it, so the assertions read the slot under test.
testing_row() {
    awk '
        $0 == "## Testing requirement" { in_section = 1; next }
        in_section && /^## / { exit }
        in_section && /^\|[[:space:]]*(yes|no)[[:space:]]*\|/ { print; exit }
    ' "$1"
}

# A plan with one goal; $2 chooses what the goal.md carries beyond the template.
#   decoy    — a hand-written yes/no table under '## Dependencies and handoffs'
#   no-row   — the testing table's data row deleted
#   plain    — the template as generated
make_plan() {
    local name="$1" variant="$2" plan goal
    export PLANS_ROOT="$work/$name"
    mkdir -p "$PLANS_ROOT"
    plan="$PLANS_ROOT/$name"
    "$scripts/create-plan.sh" "$name" "A fixture plan" >/dev/null
    "$scripts/add-goal.sh" "$plan" 01-first-goal "First goal" \
        "the renderer emits stable output" >/dev/null
    goal="$plan/01-first-goal/goal.md"
    awk -v variant="$variant" '
        /^## Dependencies and handoffs/ && variant == "decoy" {
            print; print ""
            print "| Blocking | Resolved |"
            print "|---|---|"
            print "| yes | waiting on the upstream module |"
            next
        }
        /^\| no \| <set to yes/ {
            if (variant == "no-row") next
            print "| yes | the renderer changes observable output |"
            next
        }
        { print }
    ' "$goal" > "$goal.new"
    mv "$goal.new" "$goal"
    printf '%s\n' "$plan"
}

add_unit() {
    local plan="$1" id="$2" step="$3"
    "$scripts/add-work-unit.sh" "$plan" --id "$id" --type source \
        --file "src/$id.c" --scope "symbol_$id" --subscope N/A \
        --change "change $id" --depends-on '—' --goal 01-first-goal --step "$step" \
        >/dev/null 2>&1
}

authored='| yes | the renderer changes observable output |'

# 1. The defect: the author's rationale survives a mutation of the owned-units
#    region, with an earlier yes/no table present to lose to.
plan="$(make_plan defect decoy)"
goal="$plan/01-first-goal/goal.md"
add_unit "$plan" W01 01-step-render
add_unit "$plan" W02 02-step-parse
assert_eq "$authored" "$(testing_row "$goal")" 'rationale after add-work-unit'
"$scripts/remove-work-unit.sh" "$plan" W02 >/dev/null 2>&1
assert_eq "$authored" "$(testing_row "$goal")" 'rationale after remove-work-unit'

# 2. Idempotence: consecutive mutations neither drift the row nor duplicate the
#    section the rewrite rebuilds around it.
add_unit "$plan" W03 03-step-emit
add_unit "$plan" W04 04-step-flush
assert_eq "$authored" "$(testing_row "$goal")" 'rationale after consecutive add-work-unit calls'
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$scripts/plan-document-lib.sh"
# shellcheck source=planning/scripts/plan-reconcile-lib.sh
source "$scripts/plan-reconcile-lib.sh"
inventory="$plan/work-unit-inventory.md"
plan_rewrite_owned_work_units "$goal" "$inventory" 01-first-goal
cp "$goal" "$work/once.md"
plan_rewrite_owned_work_units "$goal" "$inventory" 01-first-goal
cmp -s "$work/once.md" "$goal" || note_fail 'a second rewrite changed the goal file'
headings="$(grep -c '^## Testing requirement' "$goal" || true)"
assert_eq 1 "$headings" 'testing-requirement heading count after two rewrites'

# 3. No row in the section: the default is written and nothing fails.
plan="$(make_plan default no-row)"
goal="$plan/01-first-goal/goal.md"
add_unit "$plan" W01 01-step-render
if plan_rewrite_owned_work_units "$goal" "$plan/work-unit-inventory.md" 01-first-goal; then
    assert_eq '| no | <rationale> |' "$(testing_row "$goal")" 'default row with no row present'
else
    note_fail 'plan_rewrite_owned_work_units failed on a goal with no testing row'
fi

# 4/5. The validation pass: the decoy table is reported, the plain goal is not.
if ! command -v jq >/dev/null 2>&1; then
    printf 'goal-testing-row: UNCONFIGURED (jq) — validation assertions skipped\n'
    [ "$(t_failures)" -eq 0 ] || exit 1
    printf 'goal-testing-row: PASS\n'
    exit 0
fi
marker='hand-written yes/no table'
plan="$(make_plan reported decoy)"
add_unit "$plan" W01 01-step-render
report="$("$scripts/validate-plan.sh" "$plan" 2>&1 || true)"
assert_contains "$report" "$marker" 'unregistered table is reported'
assert_contains "$report" '## Dependencies and handoffs' 'finding names the offending section'
assert_contains "$report" 'update-plan-content.sh' 'finding names the rebuild helper'
# A hand-edited plan artifact is damage, not advice: the finding must be a FAIL,
# so it counts toward the gate's error total rather than scrolling past.
severity="$(printf '%s\n' "$report" | { grep -- "$marker" || true; } | head -1)"
case "$severity" in
    FAIL:*) ;;
    *) note_fail "unregistered table must be reported as a FAIL, got: $severity" ;;
esac

plan="$(make_plan clean plain)"
add_unit "$plan" W01 01-step-render
report="$("$scripts/validate-plan.sh" "$plan" 2>&1 || true)"
assert_not_contains "$report" "$marker" 'registered table only is not reported'

# 6. A skill tree without the registry must say so, not silently stop checking:
#    a hand-copied skill directory is exactly how a registry goes missing.
copy="$work/skill-without-registry"
mkdir -p "$copy"
( cd "$root/planning" && tar cf - scripts placeholders.json state-change-registry.json \
    never-executable-extensions.json ) | ( cd "$copy" && tar xf - )
report="$("$copy/scripts/validate-plan.sh" "$plan" 2>&1 || true)"
assert_contains "$report" 'goal-tables.json registry is missing' 'absent registry is reported'

[ "$(t_failures)" -eq 0 ] || exit 1
printf 'goal-testing-row: PASS\n'
