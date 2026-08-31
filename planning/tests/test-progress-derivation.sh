#!/usr/bin/env bash
# MODE: DEV
# test-progress-derivation.sh — every progress-tracker builder must derive the
# row Description from source intent, never write a literal placeholder into a
# generated table. The derivation is duplicated across four builders, so each
# is asserted independently:
#   - create-progress.sh        (goal-level, step -> §4.1 Objective)
#   - plan-mutate.sh rebuild-progress (goal-level, step -> §4.1 Objective)
#   - create-plan-progress.sh   (plan-level, goal -> Outcome/DoD)
#   - rebuild-plan-progress.sh  (plan-level, goal -> Outcome/DoD)
# A step/goal with a real Objective/DoD must surface that text (truncated to
# 100 chars); one with none falls back to its name, never "<short description>",
# which would fail plan validation.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-derivation-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

FAILED=0
check() { # check <desc> <file> <pattern>
    local desc="$1" file="$2" pattern="$3"
    if ! grep -Eq "$pattern" "$file"; then
        echo "FAIL: $desc" >&2
        echo "  expected pattern: $pattern" >&2
        echo "  in: $file" >&2
        sed 's/^/  | /' "$file" >&2
        FAILED=1
    fi
}
check_no_placeholder() { # check_no_placeholder <desc> <file>
    local desc="$1" file="$2"
    if grep -Fq '<short description>' "$file"; then
        echo "FAIL: $desc still contains '<short description>'" >&2
        FAILED=1
    fi
}

# ---- goal-level: create-progress.sh derives from step §4.1 Objective ----
goal_dir="$temporary_root/01-derive"
mkdir -p "$goal_dir/steps"
printf '# Goal: Derive\n' > "$goal_dir/goal.md"
printf '# Step: derive\n\n## Objective\n\n§ 4.1\n\nShip the derived objective text here.\n' > "$goal_dir/steps/01-step-derived.md"
# A step with no Objective falls back to its name, never a placeholder.
printf '# Step: bare\n' > "$goal_dir/steps/02-step-bare.md"

"$script_dir/create-progress.sh" "$goal_dir" derive >/dev/null
check 'create-progress derives from §4.1 Objective' "$goal_dir/progress.md" 'Ship the derived objective text here\.'
check_no_placeholder 'create-progress does not emit a placeholder' "$goal_dir/progress.md"

# ---- goal-level: plan-mutate.sh rebuild-progress derives the same way ----
goal_dir2="$temporary_root/02-mutate"
mkdir -p "$goal_dir2/steps"
printf '# Goal: Mutate\n' > "$goal_dir2/goal.md"
printf '# Step: mutate\n\n## Objective\n\n§ 4.1\n\nMutate-derived objective row.\n' > "$goal_dir2/steps/01-step-mutated.md"
# plan-mutate rebuild-progress requires the plan dir under git (plan_git_snapshot).
git -C "$temporary_root" init -q 2>/dev/null || true
"$script_dir/plan-mutate.sh" rebuild-progress "$goal_dir2" >/dev/null 2>&1 || {
    # plan-mutate sources plan-document-lib which may need a plans root; retry with one
    git -C "$temporary_root" init -q 2>/dev/null || true
    "$script_dir/plan-mutate.sh" rebuild-progress "$goal_dir2" >/dev/null
}
check 'plan-mutate rebuild-progress derives from §4.1 Objective' "$goal_dir2/progress.md" 'Mutate-derived objective row\.'
check_no_placeholder 'plan-mutate rebuild-progress does not emit a placeholder' "$goal_dir2/progress.md"

# ---- plan-level: create-plan-progress.sh derives from goal Outcome/DoD ----
plan_root="$temporary_root/plan"
mkdir -p "$plan_root/01-goal/steps" "$plan_root/02-goal/steps"
printf '# Goal: One\n\n## Outcome and definition of done\n\n§ 3.1\n\nThe goal-level DoD derivation text.\n' > "$plan_root/01-goal/goal.md"
printf '# Step one\n' > "$plan_root/01-goal/steps/01-step-one.md"
printf '# Goal: Bare\n' > "$plan_root/02-goal/goal.md"
printf '# Step bare\n' > "$plan_root/02-goal/steps/01-step-bare.md"

"$script_dir/create-plan-progress.sh" "$plan_root" >/dev/null
check 'create-plan-progress derives from goal Outcome/DoD' "$plan_root/progress.md" 'The goal-level DoD derivation text\.'
check_no_placeholder 'create-plan-progress does not emit a placeholder' "$plan_root/progress.md"

# ---- plan-level: rebuild-plan-progress.sh derives the same way ----
"$script_dir/rebuild-plan-progress.sh" "$plan_root" >/dev/null
check 'rebuild-plan-progress derives from goal Outcome/DoD' "$plan_root/progress.md" 'The goal-level DoD derivation text\.'
check_no_placeholder 'rebuild-plan-progress does not emit a placeholder' "$plan_root/progress.md"
# A goal with no DoD falls back to its goal name, never a placeholder.
# Pipes escaped: check() greps with -E, and a leading bare | is an empty
# alternation branch. GNU grep tolerates it, BSD grep refuses outright with
# "empty (sub)expression" and the pattern never matches. PORTABILITY(ere-empty-branch).
check 'rebuild-plan-progress falls back to goal name for a bare goal' "$plan_root/progress.md" '\| 02-goal \| 02-goal \|'

[ "$FAILED" -eq 0 ] || exit 1
printf 'Progress derivation regression test passed.\n'
