#!/usr/bin/env bash
# MODE: DEV
# test-target-path-validation.sh — repo-root validation checks concrete targets.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts="$repo_root/planning/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/target-validation.XXXXXX")"
trap 'rm -rf "$work"' EXIT

subject="$work/repo"
mkdir -p "$subject/src"
printf 'target_symbol() { :; }\n' > "$subject/src/exists.sh"

plan="$work/plan"
"$scripts/create-plan.sh" "$plan" "Target validation" >/dev/null
"$scripts/add-goal.sh" "$plan" 01-targets "Targets" "Validate concrete target rows." >/dev/null
"$scripts/update-plan-content.sh" --testing-requirement "$plan" 01-targets yes \
    "The fixture includes a source unit and a verification unit." >/dev/null
"$scripts/add-work-unit.sh" "$plan" --id W01 --type source --file src/exists.sh \
    --scope target_symbol --subscope N/A --change "Touch the existing symbol." \
    --depends-on "—" --goal 01-targets --step 01-step-existing >/dev/null
"$scripts/add-work-unit.sh" "$plan" --id W02 --type verification --file N/A \
    --scope verify-target --subscope N/A --change "Verify the existing symbol target." \
    --depends-on W01 --goal 01-targets --step 02-step-verify >/dev/null
"$scripts/add-coverage.sh" "$plan" "Existing target is covered." W01 "coverage" >/dev/null
"$scripts/add-coverage.sh" "$plan" "Existing target is verified." W02 "coverage" >/dev/null
"$scripts/create-step-testing.sh" "$plan/01-targets" 01-step-existing \
    "Run validate-plan.sh --repo-root against this fixture." >/dev/null
"$scripts/create-step-testing.sh" "$plan/01-targets" 02-step-verify \
    "Run validate-plan.sh --repo-root against this fixture." >/dev/null
"$scripts/update-plan-content.sh" --decomposition-review "$plan" completed >/dev/null
"$scripts/create-adversarial-review.sh" "$plan" >/dev/null
"$scripts/update-plan-content.sh" --review-status "$plan" approved >/dev/null

rc=0
"$scripts/validate-plan.sh" --repo-root "$subject" "$plan" >/dev/null 2>"$work/valid.err" || rc=$?
t_assert_eq "existing target validates" "$rc" 0

rc=0
"$scripts/update-work-unit.sh" "$plan" W01 --file src/missing.sh >/dev/null
"$scripts/validate-plan.sh" --repo-root "$subject" "$plan" >/dev/null 2>"$work/missing.err" || rc=$?
[ "$rc" -ne 0 ] || t_fail "missing target file passed validation"
t_assert_contains "missing target message" "target file does not exist" "$(cat "$work/missing.err")"

"$scripts/update-work-unit.sh" "$plan" W01 --file src/exists.sh --scope absent_symbol >/dev/null
rc=0
"$scripts/validate-plan.sh" --repo-root "$subject" "$plan" >/dev/null 2>"$work/symbol.err" || rc=$?
[ "$rc" -ne 0 ] || t_fail "missing target symbol passed validation"
t_assert_contains "missing symbol message" "primary symbol or file scope was not found" "$(cat "$work/symbol.err")"

plan_writer="$work/writer-plan"
"$scripts/create-plan.sh" "$plan_writer" "Writer target validation" >/dev/null
"$scripts/add-goal.sh" "$plan_writer" 01-targets "Targets" "Validate writer targets." >/dev/null
rc=0
"$scripts/add-work-unit.sh" --repo-root "$subject" "$plan_writer" --id W01 --type source --file src/missing.sh \
    --scope target_symbol --subscope N/A --change "Touch a missing file." \
    --depends-on "—" --goal 01-targets --step 01-step-missing >/dev/null 2>"$work/writer-missing.err" || rc=$?
[ "$rc" -ne 0 ] || t_fail "add-work-unit accepted a missing target under --repo-root"
t_assert_contains "writer missing message" "Target file does not exist" "$(cat "$work/writer-missing.err")"

rc=0
"$scripts/add-work-unit.sh" --repo-root "$subject" "$plan_writer" --id W01 --type source --file src/exists.sh \
    --scope absent_symbol --subscope N/A --change "Touch a missing symbol." \
    --depends-on "—" --goal 01-targets --step 01-step-missing-symbol >/dev/null 2>"$work/writer-symbol.err" || rc=$?
[ "$rc" -ne 0 ] || t_fail "add-work-unit accepted a missing symbol under --repo-root"
t_assert_contains "writer symbol message" "Primary symbol or file scope was not found" "$(cat "$work/writer-symbol.err")"

t_end "test-target-path-validation"
