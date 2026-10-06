#!/usr/bin/env bash
# MODE: DEV
# Goal tracker rows carry each step's objective text. An edit to a unit
# description or a step objective must refresh the matching row and keep its
# status; validate-plan.sh must WARN on a row that drifted and name the fix;
# update-progress.sh with no flag must leave the rows alone; create-progress.sh
# must name the refresh command when it refuses.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-row-refresh-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

fail() { t_fail "$*"; }

plan="$temporary_root/plan"
goal="$plan/01-g"
"$script_dir/create-plan.sh" "$plan" refresh >/dev/null
"$script_dir/add-goal.sh" "$plan" 01-g 'G' 'an outcome' >/dev/null
"$script_dir/add-work-unit.sh" "$plan" --id W01 --type source --file a.php \
    --scope 'A::x' --subscope N/A --change 'first description' \
    --depends-on '—' --goal 01-g --step 01-step-a >/dev/null
"$script_dir/update-step.sh" "$goal" 01-step-a completed >/dev/null

# --- the reported bug: a unit description edit must reach the tracker row -----
"$script_dir/update-work-unit.sh" "$plan" W01 --description 'second description' >/dev/null
t_assert_eq "unit description edit refreshes the row, status kept" \
    "$(grep -cFx '| 01-g | 01-step-a | second description | ✅ completed |' "$goal/progress.md" || true)" 1
t_assert_eq "the stale description is gone from the row" \
    "$(grep -cF 'first description' "$goal/progress.md" || true)" 0

# --- a step objective edit (update-plan-content -sp 4.1) reaches the row ------
"$script_dir/update-plan-content.sh" -sp "$plan" 01-g/01-step-a 4.1 'third objective' >/dev/null
t_assert_eq "step objective edit refreshes the row, status kept" \
    "$(grep -cFx '| 01-g | 01-step-a | third objective | ✅ completed |' "$goal/progress.md" || true)" 1

# --- validate-plan WARNs on a drifted row and names the fix -------------------
drifted="$temporary_root/drifted-progress.md"
{
    printf '# Progress: 01-g\n\n**Progress:** `100%%  ####################  100%%` ✅\n\n'
    printf '| Goalname | Stepname | Description | Completion status |\n|---|---|---|---|\n'
    printf '| 01-g | 01-step-a | an objective nobody wrote | ✅ completed |\n'
} > "$drifted"
cp "$drifted" "$goal/progress.md"
validate_out="$temporary_root/validate.out"
"$script_dir/validate-plan.sh" "$plan" >"$validate_out" 2>&1 || true
grep -Fq 'WARN: 01-g/progress.md row for 01-step-a does not match its step objective; run update-progress.sh --rows' "$validate_out" \
    || fail "validate-plan did not warn on the drifted row: $(cat "$validate_out")"

# update-progress.sh with no flag keeps the drifted row (only the bar is recomputed)
"$script_dir/update-progress.sh" "$goal" >/dev/null
t_assert_eq "no-flag update-progress leaves the row text alone" \
    "$(grep -cF 'an objective nobody wrote' "$goal/progress.md" || true)" 1

# --- --rows re-derives the row, keeps the status, and clears the warning ------
"$script_dir/update-progress.sh" --rows "$goal" >/dev/null
t_assert_eq "--rows restores the objective and keeps the status" \
    "$(grep -cFx '| 01-g | 01-step-a | third objective | ✅ completed |' "$goal/progress.md" || true)" 1
"$script_dir/validate-plan.sh" "$plan" >"$validate_out" 2>&1 || true
if grep -Fq 'does not match its step objective' "$validate_out"; then
    fail "validate-plan still warns after --rows: $(cat "$validate_out")"
fi

# --- create-progress refuses and names the refresh command --------------------
rc=0
create_err="$temporary_root/create.err"
"$script_dir/create-progress.sh" "$goal" 01-g >/dev/null 2>"$create_err" || rc=$?
t_assert_eq "create-progress refuses an existing tracker" "$rc" 73
grep -Fq 'update-progress.sh --rows' "$create_err" || fail "refusal does not name --rows: $(cat "$create_err")"

t_end
