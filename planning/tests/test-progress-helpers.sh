#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-progress-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

goal_dir="$temporary_root/01-demo"
mkdir -p "$goal_dir/steps"
printf '# Goal: Demo\n' > "$goal_dir/goal.md"
printf '# First step\n' > "$goal_dir/steps/01-step-first.md"

"$script_dir/create-progress.sh" "$goal_dir" demo
"$script_dir/update-step.sh" "$goal_dir" 01-step-first completed
"$script_dir/create-step-testing.sh" "$goal_dir" 01-step-first 'Run the focused regression test.'

grep -Fqx '| demo | 01-step-first | <short description> | ✅ completed |' "$goal_dir/progress.md"
grep -Fqx '**Progress:** `100%  ####################  100%` ✅' "$goal_dir/progress.md"
grep -Fqx '# Verification: 01-step-first' "$goal_dir/steps/01-step-first-testing.md"
grep -Fqx 'Run the focused regression test.' "$goal_dir/steps/01-step-first-testing.md"

plan_root="$temporary_root/plan"
mkdir -p "$plan_root"
cp -R "$goal_dir" "$plan_root/01-demo"
"$script_dir/create-plan-progress.sh" "$plan_root" >/dev/null
"$script_dir/rebuild-plan-progress.sh" "$plan_root" >/dev/null
grep -Fqx '| 01-demo | <short description> | ✅ completed |' "$plan_root/progress.md"
"$script_dir/add-goal.sh" "$plan_root" 02-next 'Next goal' 'Next outcome' >/dev/null
grep -Fqx '| 02-next | <short description> | 💤 incomplete |' "$plan_root/progress.md"

review_file="$temporary_root/adversarial-review.md"
{
    printf '# Adversarial review\n\n'
    printf '## Findings\n\n'
    printf '| ID | Missing or over-broad item | Required plan change | Status | Work unit |\n'
    printf '|---|---|---|---|---|\n'
    printf '| AR-01 | Baseline. | None. | ✅ resolved | N/A |\n\n'
    printf 'No additional substantive finding remains.\n'
} > "$review_file"
"$script_dir/plan-mutate.sh" add-finding "$temporary_root" AR-02 'Helper writes must be atomic.' 'Add the focused helper regression.' resolved >/dev/null
grep -Fqx '| AR-02 | Helper writes must be atomic. | Add the focused helper regression. | ✅ resolved | N/A |' "$review_file"
if "$script_dir/plan-mutate.sh" add-finding "$temporary_root" AR-02 'Duplicate.' 'Must fail.' >/dev/null 2>&1; then
    echo 'duplicate adversarial finding unexpectedly succeeded' >&2
    exit 1
fi

"$script_dir/plan-mutate.sh" validate "$temporary_root" >/dev/null 2>&1 || true

printf 'Progress helper regression test passed.\n'
