#!/usr/bin/env bash
# MODE: DEV
# test-progress-entry-ids.sh — both readers serve the plan tracker and a goal's
# tracker, under the same ids.
#
# The bug this pins: validate-plan-propagation-lib.sh requires
# <plan>/<goal>/progress.md for the completion gate, but no compliant reader
# could serve it -- plan-context.sh rejected goal-progress:<goal> and
# plan-content.sh rejected `progress` outright. A reviewer asked to verify a
# goal tracker had to break the read discipline SKILL.md prohibits, or skip it.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts_dir="$repo_root/planning/scripts"
work="$(mktemp -d "${TMPDIR:-/tmp}/progress-ids.XXXXXX")"
trap 'rm -rf "$work"' EXIT

note_fail() { printf 'FAIL: %s\n' "$1" >&2; t_record "$1"; }

plan="$work/plan"
cp -R "$repo_root/benchmark/planning/tests/fixtures/review-lifecycle-plan" "$plan"
goal=01-lossless-finding-contract

# Each tracker names itself in its title, which is how we tell them apart.
grep -Fq "# Progress: $goal" "$plan/$goal/progress.md" \
    || note_fail 'the fixture goal tracker does not name its goal'

expect_contains() { # <label> <needle> <output>
    case "$3" in
        *"$2"*) ;;
        *) note_fail "$1 did not contain '$2'" ;;
    esac
}

ctx() { PLANNING_CONTEXT_CACHE=1 "$scripts_dir/plan-context.sh" read --plan-dir "$plan" "$@" --read-only; }

rc=0; out="$(ctx --document "goal-progress:$goal")" || rc=$?
[ "$rc" -eq 0 ] || note_fail "plan-context goal-progress exited $rc, want 0"
expect_contains 'plan-context goal-progress' "# Progress: $goal" "$out"

rc=0; out="$(ctx --document progress)" || rc=$?
[ "$rc" -eq 0 ] || note_fail "plan-context progress exited $rc, want 0"
case "$out" in
    *"# Progress: $goal"*) note_fail 'plan-context progress served the goal tracker, not the plan one' ;;
esac

rc=0; out="$("$scripts_dir/plan-content.sh" get "$plan" "goal-progress:$goal" 2>&1)" || rc=$?
[ "$rc" -eq 0 ] || note_fail "plan-content goal-progress exited $rc, want 0"
expect_contains 'plan-content goal-progress' "# Progress: $goal" "$out"

rc=0; out="$("$scripts_dir/plan-content.sh" get "$plan" progress 2>&1)" || rc=$?
[ "$rc" -eq 0 ] || note_fail "plan-content progress exited $rc, want 0"

# A bare prefix names no goal and must be refused by both, not silently
# resolved to the plan root.
for reader in context content; do
    rc=0
    if [ "$reader" = context ]; then
        ctx --document 'goal-progress:' >/dev/null 2>&1 || rc=$?
    else
        "$scripts_dir/plan-content.sh" get "$plan" 'goal-progress:' >/dev/null 2>&1 || rc=$?
    fi
    [ "$rc" -ne 0 ] || note_fail "plan-$reader accepted a goal-progress id naming no goal"
done

# check must see the goal tracker, or an edit to it goes unnoticed.
PLANNING_CONTEXT_CACHE=1 "$scripts_dir/plan-context.sh" init --plan-dir "$plan" >/dev/null 2>&1 || true
index="$(find "$plan/context" -name index.tsv 2>/dev/null | head -1)"
if [ -n "$index" ]; then
    entries="$(grep -c "^goal-progress:" "$index" || true)"
    [ "${entries:-0}" -ge 1 ] || note_fail 'the context index carries no goal-progress entry'
else
    note_fail 'context init wrote no index'
fi

# A generated tracker is a reference document: readable, never a write target.
before="$(cat "$plan/$goal/progress.md")"
"$scripts_dir/update-plan-content.sh" -ap "$plan" "goal-progress:$goal" objective 'injected' >/dev/null 2>&1 \
    && note_fail 'a goal tracker accepted a narrative write'
[ "$before" = "$(cat "$plan/$goal/progress.md")" ] \
    || note_fail 'a refused write still modified the goal tracker'

if [ "$(t_failures)" -ne 0 ]; then
    printf 'test-progress-entry-ids: %d failure(s).\n' "$(t_failures)" >&2
    exit 1
fi
printf 'test-progress-entry-ids: PASS\n'
