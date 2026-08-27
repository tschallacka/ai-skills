#!/usr/bin/env bash
# MODE: DEV
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
# Removes the row the pristine fixture already carries, not the one the previous
# pair added: every check_pair resets a/b/p from the fixture first.
check_pair remove-coverage - remove-coverage.sh 'A converted test still reports and still fails when an assertion breaks'
check_pair add-work-unit - add-work-unit.sh --id W09 --type source --file a/b.sh --scope sym \
    --subscope N/A --change 'do a thing' --depends-on -- --goal 01-plan-dir-synonym --step 09-step-new
check_pair update-work-unit - update-work-unit.sh W01 --scope 'a rescoped symbol'
check_pair remove-work-unit - remove-work-unit.sh W08
check_pair update-plan-progress - update-plan-progress.sh 01-plan-dir-synonym in-progress
check_pair rebuild-plan-progress - rebuild-plan-progress.sh
# overview-state is read-only: check_pair requires mutation, which never happens.
# Verify --plan-dir output matches positional directly.
a="$(bash "$scripts/overview-state.sh" "$work/a/plan")"
b="$(bash "$scripts/overview-state.sh" --plan-dir "$work/b/plan")"
[ "$a" = "$b" ] || t_fail 'overview-state: --plan-dir output differs from positional'
[ -n "$a" ] || t_fail 'overview-state produced no output'
# overview-serve starts a server and never exits: cannot use check_pair.
# The --plan-dir flag is verified by test-overview-serve.sh instead.
check_pair render-plan-overview - render-plan-overview.sh
check_pair create-plan-progress 'rm -f progress.md' create-plan-progress.sh
check_pair register-command - register-command.sh probe-key 'ls -la' 'while probing'
check_pair mint-fix-keys - mint-fix-keys.sh
check_pair add-adversarial-finding - add-adversarial-finding.sh AR-99 'A probe finding' 'Fix it somehow' open
check_pair create-adversarial-review 'rm -f adversarial-review.md' create-adversarial-review.sh
check_pair create-work-unit-inventory 'rm -f work-unit-inventory.md' create-work-unit-inventory.sh
check_pair create-ui-validation - create-ui-validation.sh 'http://localhost:8080'
check_pair add-ui-story "$ui_ready" add-ui-story.sh --id US-01 --persona 'a returning user' \
    --actions 'open the dashboard' --interaction 'click the refresh control' --expected 'it renders' --work-units W01
check_pair add-ui-story-links "$story_ready" add-ui-story-links.sh US-01 W02
check_pair configure-ui-story-cache "$story_ready" configure-ui-story-cache.sh --id US-01 \
    --starting-state 'logged out' --input 'click sign in' --target 'the dashboard' \
    --readiness 'the header appears' --max-wait '10s'
check_pair create-ui-story-run-cache - create-ui-story-run-cache.sh US-01
check_pair remove-plan - remove-plan.sh
# update-ui-story carries the same --plan-dir pair: it rewrites an existing
# story row (B61/B62 landed it; this pair keeps its hoisting covered).
check_pair update-ui-story "$story_ready" update-ui-story.sh US-01 --persona 'an editor' \
    --actions 'edits the story' --interaction 'types' --expected 'row updated'

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

# Scripts that read a plan and write nothing. A tree diff proves nothing there,
# so the differential is exit status plus stdout. Without this they were exempt on
# a claim that turned out to be false: no test in the suite invoked any of them
# with --plan-dir at all.
readonly_pair() { # <label> <script> <args...>
    local label="$1" script="$2"
    shift 2
    reset_copies -
    local arc=0 brc=0 aout bout
    aout="$("$scripts/$script" "$work/a/plan" "$@" 2>/dev/null)" || arc=$?
    bout="$("$scripts/$script" --plan-dir "$work/b/plan" "$@" 2>/dev/null)" || brc=$?
    # Both forms succeeding is the assertion that a differential cannot make.
    # Breaking the hoist breaks both forms the same way -- each exits 64 with a
    # usage message -- so parity alone reports nothing. These inputs return 0 on
    # the fixture, so a lost --plan-dir shows up here.
    t_assert_eq "$label: positional succeeds" "$arc" 0
    t_assert_eq "$label: --plan-dir succeeds" "$brc" 0
    t_assert_eq "$label: --plan-dir agrees on stdout" \
        "$(printf '%s' "$bout" | sed "s|$work/b|<plan>|g")" \
        "$(printf '%s' "$aout" | sed "s|$work/a|<plan>|g")"
}

# plan-content takes its subcommand before the plan directory, so the plan
# directory is not argument 1 and the plain form above cannot drive it.
readonly_sub_pair() { # <label> <script> <subcommand> <args...>
    local label="$1" script="$2" subcommand="$3"
    shift 3
    reset_copies -
    local arc=0 brc=0 aout bout
    aout="$("$scripts/$script" "$subcommand" "$work/a/plan" "$@" 2>/dev/null)" || arc=$?
    bout="$("$scripts/$script" "$subcommand" --plan-dir "$work/b/plan" "$@" 2>/dev/null)" || brc=$?
    t_assert_eq "$label: positional succeeds" "$arc" 0
    t_assert_eq "$label: --plan-dir succeeds" "$brc" 0
    t_assert_eq "$label: --plan-dir agrees on stdout" \
        "$(printf '%s' "$bout" | sed "s|$work/b|<plan>|g")" \
        "$(printf '%s' "$aout" | sed "s|$work/a|<plan>|g")"
}

# update-adversarial-review reads its rows from stdin, which check_pair cannot
# drive, so it gets its own case rather than an exemption.
review_pair() { # <label> <csv>
    local label="$1" csv="$2"
    reset_copies -
    local arc=0 brc=0
    printf '%s\n' "$csv" | "$scripts/update-adversarial-review.sh" "$work/a/plan" >/dev/null 2>&1 || arc=$?
    printf '%s\n' "$csv" | "$scripts/update-adversarial-review.sh" --plan-dir "$work/b/plan" >/dev/null 2>&1 || brc=$?
    t_assert_eq "$label: positional succeeds" "$arc" 0
    t_assert_eq "$label: --plan-dir agrees on status" "$brc" "$arc"
    t_assert_eq "$label: --plan-dir produces the same tree" \
        "$(normalise "$work/b/plan")" "$(normalise "$work/a/plan")"
}

# The four scripts the enumeration gate below found uncovered.
readonly_sub_pair plan-content plan-content.sh get plan text
readonly_pair verify-target verify-target.sh W01
readonly_pair verify-fix-keys verify-fix-keys.sh
review_pair update-adversarial-review 'AR-77,A differential finding.,Do the thing.,✅ resolved,W01'

# ---- every hoisting script has a differential case ------------------------
# The subjects above are named by hand, so a script that starts accepting
# --plan-dir is covered only when someone remembers to add it here. Four already
# were not: plan-content.sh, update-adversarial-review.sh, verify-fix-keys.sh and
# verify-target.sh. The flag-coverage test could not see the gap either, because
# a hoisted flag has no case label in the script that accepts it.
#
# Anything genuinely not differentiable belongs in the exemption list with its
# reason, not left silently uncovered.
exempt_from_pairs() { # <script>
    case "$1" in
        # A library defines the hoister or is compiled from the file that does;
        # it is the mechanism, not a caller with an argument list of its own.
        *-lib.sh) return 0 ;;
        # overview-state is read-only (no tree mutation to verify) and
        # overview-serve starts a server that never exits; both are covered
        # by dedicated checks elsewhere in this file or in their own tests.
        overview-state.sh|overview-serve.sh) return 0 ;;
    esac
    return 1
}

# Subjects come from the invocations, not from any mention of the name. Grepping
# the whole file counted the four scripts listed in the comment above as covered,
# so this gate passed on its own prose -- the failure mode CODE-STYLE.md section
# 12 describes, in the check written to prevent a different one.
# The script is the first *.sh token on a check_pair line, matched rather than
# taken by field number: a quoted precondition such as 'rm -f progress.md'
# contains spaces, so $4 is "-f" and three covered scripts read as uncovered.
named_subjects="$(
    grep '^check_pair ' "${BASH_SOURCE[0]}" | grep -oE '[a-z0-9-]+\.sh'
    grep -E '^(readonly_pair|readonly_sub_pair|review_pair) ' "${BASH_SOURCE[0]}" | grep -oE '[a-z0-9-]+\.sh'
    grep -q '^review_pair ' "${BASH_SOURCE[0]}" && printf 'update-adversarial-review.sh\n'
    grep -q '^upc_pair ' "${BASH_SOURCE[0]}" && printf 'update-plan-content.sh\n'
)"
newline='
'
newline_wrapped_subjects="$newline$named_subjects$newline"
while IFS= read -r hoisting; do
    [ -n "$hoisting" ] || continue
    exempt_from_pairs "$hoisting" && continue
    case "$newline_wrapped_subjects" in
        *"$newline$hoisting$newline"*) ;;
        *) t_fail "$hoisting accepts --plan-dir through the hoister but has no case here; add one or exempt it with a reason" ;;
    esac
done <<HOISTING
$(grep -l 'plan_hoist_plan_dir' "$scripts"/*.sh | while IFS= read -r found; do basename "$found"; done)
HOISTING

t_end
