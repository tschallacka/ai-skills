#!/usr/bin/env bash
# test-target-reachability-gate.sh — verify-target.sh must check, or fail closed.
#
# The gate used to run its reachability checks for `markup` and `style` units
# only: a template recorded as `source`, and every `discovery` unit, exited 0
# having checked nothing while SKILL.md presents this script as the static half
# of the reachability gate. These assertions pin the replacement rule — the
# target file decides what runs, and a check that cannot run fails.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-target-gate-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

pass() { printf 'PASS: %s\n' "$1"; }
fail() { printf 'FAIL: %s\n' "$1" >&2; t_record "$1"; }

# assert_exit <expected> <label> <log> -- <command…>
assert_exit() {
    local expected="$1" label="$2" log="$3" code=0
    shift 4
    "$@" > "$log" 2>&1 || code="$?"
    if [ "$code" = "$expected" ]; then
        pass "$label"
    else
        fail "$label (expected exit $expected, got $code)"
        sed 's/^/    /' "$log" >&2
    fi
}

assert_log() {
    local log="$1" needle="$2" label="$3"
    if grep -Fq "$needle" "$log"; then
        pass "$label"
    else
        fail "$label (log has no '$needle')"
        sed 's/^/    /' "$log" >&2
    fi
}

plan="$temporary_root/plan"
repo="$temporary_root/repo"
mkdir -p "$repo/app/code/Fake/Module/view/frontend/templates" \
    "$repo/app/code/Fake/Module/view/frontend/layout" \
    "$repo/app/code/Fake/Module/Model"
printf 'rendered\n' > "$repo/app/code/Fake/Module/view/frontend/templates/history.phtml"
printf '<?php class Thing {}\n' > "$repo/app/code/Fake/Module/Model/Thing.php"

"$script_dir/create-plan.sh" "$plan" 'Reachability fixture' >/dev/null
"$script_dir/add-goal.sh" "$plan" 01-goal 'Goal' 'Outcome' >/dev/null
# A template recorded as `source`, a discovery unit over the same template, a
# unit with no target at all, and a plain PHP class: the shapes the type-gated
# check waved through.
"$script_dir/add-work-unit.sh" "$plan" W01 source \
    app/code/Fake/Module/view/frontend/templates/history.phtml '#order_history' N/A \
    'edit the template' '—' 01-goal 01-step-template >/dev/null
"$script_dir/add-work-unit.sh" "$plan" W02 verification N/A 'reachability evidence' N/A \
    'record the render route' '—' 01-goal 01-step-discovery >/dev/null
"$script_dir/add-work-unit.sh" "$plan" W03 source \
    app/code/Fake/Module/Model/Thing.php 'Thing::run()' N/A \
    'edit the class' '—' 01-goal 01-step-class >/dev/null
"$script_dir/add-work-unit.sh" "$plan" W04 source \
    app/code/Fake/Module/view/frontend/templates/history.phtml N/A N/A \
    'edit the template blind' '—' 01-goal 01-step-blind >/dev/null
"$script_dir/add-work-unit.sh" "$plan" W05 discovery \
    app/code/Fake/Module/view/frontend/templates/history.phtml '#order_history' N/A \
    'record which route renders the template' '—' 01-goal 01-step-route >/dev/null

log="$temporary_root/w01.log"
assert_exit 0 'a source-typed template passes when nothing contradicts it' "$log" -- \
    "$script_dir/verify-target.sh" "$plan" W01 --repo "$repo"
assert_log "$log" 'layout remove/re-point' 'the result line names the reachability checks that ran'

printf '<layout><referenceBlock name="order_history" remove="true"/></layout>\n' \
    > "$repo/app/code/Fake/Module/view/frontend/layout/catalog.xml"
log="$temporary_root/w01-removed.log"
assert_exit 1 'a source-typed template fails when a layout removes its block' "$log" -- \
    "$script_dir/verify-target.sh" "$plan" W01 --repo "$repo"
assert_log "$log" 'a layout removes block' 'the removal is reported, not skipped'

log="$temporary_root/w02.log"
assert_exit 1 'a unit with no target fails closed' "$log" -- \
    "$script_dir/verify-target.sh" "$plan" W02 --repo "$repo"
assert_log "$log" 'no target file recorded' 'the no-target refusal says why nothing could be checked'

log="$temporary_root/w03.log"
assert_exit 0 'a non-render-surface target still gets the existence check' "$log" -- \
    "$script_dir/verify-target.sh" "$plan" W03 --repo "$repo"
assert_log "$log" 'is not a render surface' 'a non-surface target says which checks do not apply'
assert_log "$log" 'OK target file exists' 'the existence check ran for the non-surface target'

rm -f "$repo/app/code/Fake/Module/Model/Thing.php"
log="$temporary_root/w03-missing.log"
assert_exit 1 'a missing non-render-surface target fails' "$log" -- \
    "$script_dir/verify-target.sh" "$plan" W03 --repo "$repo"
assert_log "$log" 'target file does not exist' 'the missing file is reported for a source unit'

log="$temporary_root/w05.log"
assert_exit 1 'a discovery unit inherits the same layout-removal failure' "$log" -- \
    "$script_dir/verify-target.sh" "$plan" W05 --repo "$repo"
assert_log "$log" 'a layout removes block' 'a discovery unit runs the reachability checks too'

log="$temporary_root/w04.log"
assert_exit 1 'a render surface with no block name in scope fails closed' "$log" -- \
    "$script_dir/verify-target.sh" "$plan" W04 --repo "$repo"
assert_log "$log" 'has no block name' 'the missing block name says the checks could not run'

# No type may exit 0 with nothing checked: the old skip line is gone for good.
for unit in W01 W02 W03 W04 W05; do
    verdict_out="$("$script_dir/verify-target.sh" "$plan" "$unit" --repo "$repo" 2>/dev/null || true)"
    case "$verdict_out" in
        *SKIP*) fail "$unit still reports a SKIP verdict" ;;
    esac
done
pass 'no unit reports the old SKIP verdict'

if [ "$(t_failures)" -ne 0 ]; then
    printf 'test-target-reachability-gate: %d failure(s).\n' "$(t_failures)" >&2
    exit 1
fi
printf 'test-target-reachability-gate.sh passed.\n'
