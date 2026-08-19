#!/usr/bin/env bash
# test-flag-form-equivalence — the named-flag form of the helpers that used to
# take many positional arguments must produce byte-identical documents to the
# deprecated positional form.
#
# Usage: test-flag-form-equivalence.sh
#
# add-work-unit.sh took 10 positional arguments, add-ui-story.sh and
# configure-ui-story-cache.sh took 7. They now accept named flags, and the
# positional form is kept working for existing callers (plan-mutate.sh, SKILL.md
# and the other tests all still use it). This test builds the same plan twice —
# once each way — and diffs the trees, so the deprecated path cannot silently
# drift away from the documented one. It also covers add-adversarial-finding.sh's
# --status and --work-unit, the latter being the gated form that mints fix keys.
set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-flag-form.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

fail() { printf 'flag-form: %s\n' "$1" >&2; exit 1; }

# Build one plan. $2 selects the argument style, so the two runs differ in
# nothing else.
build_plan() {
    local plan_dir="$1" style="$2"

    "$script_dir/create-plan.sh" "$plan_dir" 'Flag form equivalence' >/dev/null
    "$script_dir/add-goal.sh" "$plan_dir" 01-render 'Render the control' \
        'The control renders and responds.' >/dev/null

    if [ "$style" = flags ]; then
        "$script_dir/add-work-unit.sh" "$plan_dir" \
            --id W01 --type markup --file app/index.html \
            --scope '#submit' --subscope N/A \
            --change 'Add the submit control.' --depends-on '—' \
            --goal 01-render --step 01-step-add-control >/dev/null 2>&1
        "$script_dir/add-work-unit.sh" "$plan_dir" \
            --id W02 --type verification --file N/A \
            --scope 'validate-plan.sh' --subscope N/A \
            --change 'Verify the control renders.' --depends-on W01 \
            --goal 01-render --step 02-step-verify-control >/dev/null 2>&1
    else
        "$script_dir/add-work-unit.sh" "$plan_dir" W01 markup app/index.html \
            '#submit' N/A 'Add the submit control.' '—' \
            01-render 01-step-add-control >/dev/null 2>&1
        "$script_dir/add-work-unit.sh" "$plan_dir" W02 verification N/A \
            'validate-plan.sh' N/A 'Verify the control renders.' W01 \
            01-render 02-step-verify-control >/dev/null 2>&1
    fi

    "$script_dir/create-ui-validation.sh" "$plan_dir" \
        'Serve index.html from a local static HTTP server.' >/dev/null

    if [ "$style" = flags ]; then
        "$script_dir/add-ui-story.sh" "$plan_dir" \
            --id US-01 --persona 'A visitor sees the submit control.' \
            --actions 'Open the page and click the control.' \
            --interaction 'Mouse click on the control.' \
            --expected 'The confirmation is visible.' \
            --work-units W01,W02 >/dev/null
        "$script_dir/configure-ui-story-cache.sh" "$plan_dir" \
            --id US-01 --starting-state 'The served page shows the control.' \
            --input 'Mouse click on the control.' --target 'The control.' \
            --readiness 'The confirmation is visible.' --max-wait '2 s' >/dev/null
    else
        "$script_dir/add-ui-story.sh" "$plan_dir" US-01 \
            'A visitor sees the submit control.' \
            'Open the page and click the control.' \
            'Mouse click on the control.' \
            'The confirmation is visible.' W01,W02 >/dev/null
        "$script_dir/configure-ui-story-cache.sh" "$plan_dir" US-01 \
            'The served page shows the control.' \
            'Mouse click on the control.' 'The control.' \
            'The confirmation is visible.' '2 s' >/dev/null
    fi
}

flag_plan="$temporary_root/by-flag/plan"
positional_plan="$temporary_root/by-positional/plan"
mkdir -p "$temporary_root/by-flag" "$temporary_root/by-positional"
PLANS_ROOT="$temporary_root/by-flag" build_plan "$flag_plan" flags
PLANS_ROOT="$temporary_root/by-positional" build_plan "$positional_plan" positional

# The plan-description title line embeds nothing environment-specific, and no
# helper records a path or timestamp, so the two trees must match exactly. .git
# is excluded because commit ids and times legitimately differ.
if ! diff -r -x '.git' -x '.env' "$positional_plan" "$flag_plan" >"$temporary_root/tree.diff" 2>&1; then
    printf 'flag-form: flag and positional forms produced different documents:\n' >&2
    cat "$temporary_root/tree.diff" >&2
    exit 1
fi
printf '%s\n' 'flag-form: add-work-unit/add-ui-story/configure-ui-story-cache flag == positional'

# A flag form that omits a required value must fail with the usage code, not
# silently write a half-populated row.
set +e
"$script_dir/add-work-unit.sh" "$flag_plan" --id W03 --type source >/dev/null 2>&1
rc=$?
set -e
[ "$rc" -eq 64 ] || fail "an incomplete flag form exited $rc, expected 64"

# add-adversarial-finding: --status, then --work-unit as the gated form.
"$script_dir/create-adversarial-review.sh" "$flag_plan" >/dev/null
"$script_dir/add-adversarial-finding.sh" "$flag_plan" AR-02 \
    'The control has no keyboard path.' 'Add keyboard activation.' \
    --status in-progress >/dev/null 2>&1
grep -Eq '^\|[[:space:]]*AR-02[[:space:]]*\|.*in progress' "$flag_plan/adversarial-review.md" \
    || fail '--status in-progress did not land in the findings table'

"$script_dir/add-adversarial-finding.sh" "$flag_plan" AR-03 \
    'The control is not verified.' 'Add a verification unit.' \
    --work-unit W02 >/dev/null 2>&1
grep -Eq '^\|[[:space:]]*AR-03[[:space:]]*\|.*\|[[:space:]]*W02[[:space:]]*\|' \
    "$flag_plan/adversarial-review.md" \
    || fail '--work-unit did not record the gated work unit'
[ -f "$flag_plan/fix-keys.json" ] \
    || fail '--work-unit did not mint fix keys for the gated finding'
grep -Fq 'AR-03' "$flag_plan/fix-keys.json" \
    || fail 'fix-keys.json does not carry the gated finding id'
printf '%s\n' 'flag-form: add-adversarial-finding --status and --work-unit gate correctly'

# --- --plan-dir is a synonym for the positional plan directory ----------------
# plan-context.sh and run-adversary-probe.sh take the flag, so a reader who
# learned it there must not have the call refused by the other tools.
plan_dir_synonym() { # <label> <script> <args-before-dir...> -- <args-after-dir...>
    local label="$1" script="$2"; shift 2
    local before=() after=() seen_sep=0 arg
    for arg in "$@"; do
        if [ "$arg" = -- ] && [ "$seen_sep" -eq 0 ]; then seen_sep=1; continue; fi
        if [ "$seen_sep" -eq 0 ]; then before+=("$arg"); else after+=("$arg"); fi
    done
    local pos_out flag_out pos_rc=0 flag_rc=0
    # PORTABILITY(empty-array-setu)
    pos_out="$("$script_dir/$script" ${before[@]+"${before[@]}"} "$fixture" ${after[@]+"${after[@]}"} 2>&1)" || pos_rc=$?
    flag_out="$("$script_dir/$script" ${before[@]+"${before[@]}"} --plan-dir "$fixture" ${after[@]+"${after[@]}"} 2>&1)" || flag_rc=$?
    [ "$pos_rc" -eq "$flag_rc" ] \
        || fail "$label: positional exited $pos_rc but --plan-dir exited $flag_rc"
    [ "$pos_out" = "$flag_out" ] \
        || fail "$label: --plan-dir produced different output from the positional form"
}

fixture="$(cd "$script_dir/../../benchmark/planning/tests/fixtures/review-lifecycle-plan" && pwd)"
plan_dir_synonym 'validate-plan'            validate-plan.sh   --
plan_dir_synonym 'validate-plan --complete' validate-plan.sh   --complete --
plan_dir_synonym 'verify-fix-keys'          verify-fix-keys.sh --
plan_dir_synonym 'verify-target'            verify-target.sh   -- W01
plan_dir_synonym 'plan-content get'         plan-content.sh    get -- plan
plan_dir_synonym 'plan-content summary'     plan-content.sh    summary --
plan_dir_synonym 'plan-content find'        plan-content.sh    find -- Findings
printf '%s\n' 'flag-form: --plan-dir matches the positional plan directory'

printf '%s\n' 'test-flag-form-equivalence: PASS'
