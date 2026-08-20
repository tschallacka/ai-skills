#!/usr/bin/env bash
# MODE: DEV
# test-plan-context-unit-entry.sh — a work-unit entry is served from its step
# file AND its inventory row, and a view that cannot apply refuses.
#
# The bugs this pins: --unit read the inventory row only to locate the step file
# and then discarded it, so `--view dependencies` returned exit 0 with zero
# bytes -- silence indistinguishable from "no dependencies". The entry hash
# covered the step file alone, so rewriting a unit's row did not mark it
# changed. And context_die returns rather than exits, so a refusing view ran on
# and reported its follow-on failure instead of the refusal.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts_dir="$repo_root/planning/scripts"
fixture="$repo_root/benchmark/planning/tests/fixtures/review-lifecycle-plan"
work="$(mktemp -d "${TMPDIR:-/tmp}/plan-context-unit.XXXXXX")"
trap 'rm -rf "$work"' EXIT

note_fail() { printf 'FAIL: %s\n' "$1" >&2; t_record "$1"; }

plan="$work/plan"
cp -R "$fixture" "$plan"

read_unit() {
    PLANNING_CONTEXT_CACHE=1 "$scripts_dir/plan-context.sh" read \
        --plan-dir "$plan" "$@" --read-only 2>&1
}

# ---- the row is reachable -----------------------------------------------------
row=""
read_rc=0
row="$(read_unit --unit W01 --view inventory-row)" || read_rc=$?
[ "$read_rc" -eq 0 ] || note_fail "inventory-row on a work unit exited $read_rc, want 0"
case "$row" in
    *'## Inventory row'*) ;;
    *) note_fail 'inventory-row view did not serve the row block' ;;
esac
for field in '- ID: W01' '- Type:' '- Depends on:' '- Goal:' '- Step:'; do
    case "$row" in
        *"$field"*) ;;
        *) note_fail "inventory-row view omitted '$field'" ;;
    esac
done

# ---- dependencies can no longer answer with silence --------------------------
deps=""
read_rc=0
deps="$(read_unit --unit W01 --view dependencies)" || read_rc=$?
[ "$read_rc" -eq 0 ] || note_fail "dependencies on a work unit exited $read_rc, want 0"
[ -n "$deps" ] || note_fail 'dependencies view returned nothing for a work unit'
case "$deps" in
    *'Depends on'*) ;;
    *) note_fail 'dependencies view did not name the row Depends-on field' ;;
esac

summary=""
read_rc=0
summary="$(read_unit --unit W01 --view execution-summary)" || read_rc=$?
[ "$read_rc" -eq 0 ] || note_fail "execution-summary on a work unit exited $read_rc, want 0"
for field in '## Execution summary' '## Inventory' '## Acceptance criteria' '## Dependencies' '## Testing' '## Status' '- File:' '- Goal:'; do
    case "$summary" in
        *"$field"*) ;;
        *) note_fail "execution-summary omitted '$field'" ;;
    esac
done

# ---- a view that cannot apply refuses, with the documented code --------------
rc=0
out="$(read_unit --document plan --view inventory-row)" || rc=$?
[ "$rc" -eq 64 ] || note_fail "inventory-row on a non-unit exited $rc, want 64"
case "$out" in
    *'applies only to a work unit'*) ;;
    *) note_fail 'the refusal did not say why the view cannot apply' ;;
esac

# The testing view refuses the same way, and must not leak its follow-on awk
# failure -- context_die returns, so the case body has to stop itself.
rc=0
out="$(read_unit --document plan --view testing)" || rc=$?
[ "$rc" -eq 64 ] || note_fail "testing view on a companion-less document exited $rc, want 64"
case "$out" in
    *'cannot open'*) note_fail 'the testing refusal leaked its follow-on awk error' ;;
esac

rc=0
out="$(read_unit --unit W01 --view full 2>&1)" || rc=$?
[ "$rc" -eq 0 ] || note_fail "full view on a work unit exited $rc, want 0"
case "$out" in
    *'returned_records='*'total_records='*'truncated='*) ;;
    *) note_fail 'text read output omitted record-count metadata' ;;
esac

rc=0
out="$(read_unit --unit W01 --view not-a-view)" || rc=$?
[ "$rc" -eq 64 ] || note_fail "unsupported view exited $rc, want 64"
case "$out" in
    *'valid views are'*'--view summary'*'--view full'*) ;;
    *) note_fail 'unsupported-view refusal did not list valid values and recovery guidance' ;;
esac

# ---- the entry hash covers every input --------------------------------------
hash_entry() {
    "$BASH" -c '
        set -euo pipefail
        source "$1/plan-map-lib.sh"; source "$1/plan-document-lib.sh"
        source "$1/plan-inventory-lib.sh"; source "$1/plan-context-lib.sh"
        context_hash_entry "$2" "$3"
    ' _ "$scripts_dir" "$plan" "$2"
}

before="$(hash_entry _ unit:W01)"
step_before="$(hash_entry _ 'step:01-lossless-finding-contract/01-step-preserve-finding-envelope')"
printf '\n' >> "$plan/work-unit-inventory.md"
after="$(hash_entry _ unit:W01)"
[ "$before" != "$after" ] \
    || note_fail 'editing the inventory did not change the work unit entry hash'

# A single-input entry must keep the plain file hash, so existing entries and
# any outstanding token are untouched by the composite.
plain="$("$BASH" -c '
    set -euo pipefail
    source "$1/plan-map-lib.sh"; source "$1/plan-document-lib.sh"
    source "$1/plan-inventory-lib.sh"; source "$1/plan-context-lib.sh"
    context_hash_file "$2/plan-description.md"
' _ "$scripts_dir" "$plan")"
[ "$(hash_entry _ plan)" = "$plain" ] \
    || note_fail 'a single-input entry hash diverged from the plain file hash'
[ -n "$step_before" ] || note_fail 'step entry hash was empty'

if [ "$(t_failures)" -ne 0 ]; then
    printf 'test-plan-context-unit-entry: %d failure(s).\n' "$(t_failures)" >&2
    exit 1
fi
printf 'test-plan-context-unit-entry: PASS\n'
