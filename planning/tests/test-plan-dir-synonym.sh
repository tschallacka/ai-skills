#!/usr/bin/env bash
# test-plan-dir-synonym.sh — every helper that takes a plan directory accepts
# --plan-dir, and the two forms do the same thing.
#
# plan-context.sh and run-adversary-probe.sh take the flag natively; the rest take
# the directory positionally and reach the same place through plan_hoist_plan_dir.
# A reader who learned the flag from the bounded reader must not have the call
# refused elsewhere.
#
# Each case asserts three things, and the third is the one that matters: the
# operation HAD AN EFFECT. Comparing two invocations for equal output proves
# nothing if both failed the same way -- an earlier version of this harness
# reported "identical" for a dozen helpers that were all broken at load time.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts="$repo_root/planning/scripts"
fixture="$repo_root/benchmark/planning/tests/fixtures/self-hosted-plan"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/plan-dir-synonym.XXXXXX")"
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/a" "$work/b" "$work/p"

# The plan's own directory name appears in generated titles, so both copies must
# be named the same or the trees differ for that reason alone.
reset_copies() {
    local v
    for v in a b p; do
        rm -rf "$work/$v/plan"
        cp -R "$fixture" "$work/$v/plan"
        [ "${1:-}" = - ] || [ ! -d "$work/$v/plan" ] || \
            ( cd "$work/$v/plan" && eval "$1" ) >/dev/null 2>&1
    done
}

# Minted ids and timestamps differ per run by design.
normalise() {
    # A read loop rather than xargs -r, which is a GNU extension.
    ( cd "$1" && find . -type f | LC_ALL=C sort | while IFS= read -r f; do
        printf '==== %s\n' "$f"
        sed 's/[0-9a-f]\{16\}/<id>/g; s/[0-9]\{4\}-[0-9][0-9]-[0-9][0-9]T[0-9:]*Z\{0,1\}/<ts>/g' "$f"
    done )
}

check_pair() { # <label> <precondition|-> <script> <args...>
    local label="$1" pre="$2" script="$3"
    shift 3
    reset_copies "$pre"
    local arc=0 brc=0
    "$scripts/$script" "$work/a/plan" "$@" >/dev/null 2>&1 || arc=$?
    "$scripts/$script" --plan-dir "$work/b/plan" "$@" >/dev/null 2>&1 || brc=$?
    t_assert_eq "$label: positional succeeds" "$arc" 0
    t_assert_eq "$label: --plan-dir agrees on status" "$brc" "$arc"
    # remove-plan deletes the tree, so there is nothing to normalise: the effect
    # to assert is that both forms removed it.
    if [ ! -d "$work/a/plan" ] || [ ! -d "$work/b/plan" ]; then
        if [ -d "$work/a/plan" ] || [ -d "$work/b/plan" ]; then
            t_fail "$label: one argument form removed the plan and the other did not"
        fi
        return 0
    fi
    if ! diff <(normalise "$work/a/plan") <(normalise "$work/b/plan") >/dev/null 2>&1; then
        t_fail "$label: the two argument forms produced different trees"
    fi
    if diff -r "$work/p/plan" "$work/a/plan" >/dev/null 2>&1; then
        t_fail "$label: the invocation changed nothing, so it proves nothing"
    fi
}

ui_ready='"$SCRIPTS/create-ui-validation.sh" . http://localhost:8080'
story_ready="$ui_ready"' && "$SCRIPTS/add-ui-story.sh" . --id US-01 --persona p --actions a --interaction "click it" --expected e --work-units W01'
export SCRIPTS="$scripts"

check_pair add-goal - add-goal.sh 03-new-goal 'A third goal' 'Something verifiable happens.'
check_pair add-coverage - add-coverage.sh 'Another required proof' W01 'A note.'
check_pair add-work-unit - add-work-unit.sh --id W09 --type source --file a/b.sh --scope sym \
    --subscope N/A --change 'do a thing' --depends-on -- --goal 01-plan-dir-synonym --step 09-step-new
check_pair update-work-unit - update-work-unit.sh W01 --scope 'a rescoped symbol'
check_pair remove-work-unit - remove-work-unit.sh W08
check_pair update-plan-progress - update-plan-progress.sh 01-plan-dir-synonym in-progress
check_pair rebuild-plan-progress - rebuild-plan-progress.sh
check_pair create-plan-progress 'rm -f progress.md' create-plan-progress.sh
check_pair register-command - register-command.sh probe-key 'ls -la' 'while probing'
check_pair mint-fix-keys - mint-fix-keys.sh
check_pair add-adversarial-finding - add-adversarial-finding.sh AR-99 'A probe finding' 'Fix it somehow' open
check_pair create-adversarial-review 'rm -f adversarial-review.md' create-adversarial-review.sh
check_pair create-work-unit-inventory 'rm -f work-unit-inventory.md' create-work-unit-inventory.sh
check_pair create-ui-validation - create-ui-validation.sh 'http://localhost:8080'
check_pair add-ui-story "$ui_ready" add-ui-story.sh --id US-01 --persona 'a returning user' \
    --actions 'open the dashboard' --interaction 'click the refresh control' --expected 'it renders' --work-units W01
check_pair configure-ui-story-cache "$story_ready" configure-ui-story-cache.sh --id US-01 \
    --starting-state 'logged out' --input 'click sign in' --target 'the dashboard' \
    --readiness 'the header appears' --max-wait '10s'
check_pair create-ui-story-run-cache - create-ui-story-run-cache.sh US-01
check_pair remove-plan - remove-plan.sh

# update-plan-content.sh takes its subcommand before the plan directory, so the
# hoist sits after that shift and the pair harness above cannot express it.
upc_pair() { # <label> <flag> <args-after-plan-dir...>
    local label="$1" flag="$2"
    shift 2
    reset_copies -
    local arc=0 brc=0
    "$scripts/update-plan-content.sh" "$flag" "$work/a/plan" "$@" >/dev/null 2>&1 || arc=$?
    "$scripts/update-plan-content.sh" "$flag" --plan-dir "$work/b/plan" "$@" >/dev/null 2>&1 || brc=$?
    t_assert_eq "update-plan-content $label: positional succeeds" "$arc" 0
    t_assert_eq "update-plan-content $label: --plan-dir agrees" "$brc" "$arc"
    diff -r "$work/a/plan" "$work/b/plan" >/dev/null 2>&1 \
        || t_fail "update-plan-content $label: the two forms produced different trees"
    if diff -r "$work/p/plan" "$work/a/plan" >/dev/null 2>&1; then
        t_fail "update-plan-content $label: the invocation changed nothing"
    fi
}

upc_pair -dp -dp 2.1 'a rewritten paragraph'
upc_pair -ap -ap plan current-state 'an appended paragraph'
upc_pair -f -f plan 'UI affected' yes

t_end
