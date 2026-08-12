#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/plan-document-lib.sh"
source "$SCRIPT_DIR/plan-reconcile-lib.sh"

HELP=$'Removes a work unit and reconciles every reference so you do not need to\nfollow up by hand.\n\nUsage: remove-work-unit.sh <plan-directory> <WNN>\n\nRemoves: the inventory row; the id from coverage rows (others kept; row dropped\nonly if empty); the goal Owned work units section (re-derived); the step file\nand its -testing companion; and the goal + plan progress trackers (rebuilt).'
plan_guard 2 "$HELP" "$@"
plan_dir="$1"; unit="$2"

plan_require_directory "$plan_dir" || plan_err "plan directory not found: $plan_dir (pass an absolute path, e.g. .plans/<plan-name>)"
[[ "$unit" =~ ^W[0-9][0-9]+$ ]] || plan_err "invalid work-unit id '$unit' — must be WNN (e.g. W01, W02)"
inventory="$plan_dir/work-unit-inventory.md"
[ -f "$inventory" ] || plan_err "work-unit inventory not found: $inventory (the plan appears incomplete)"

# Locate the inventory row.
goal=''; step=''
if IFS=$'\t' read -r goal step < <(awk -F'|' -v wanted="$unit" '
    /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {
        id=$2; gsub(/^[[:space:]]+|[[:space:]]+$/, "", id)
        if (id == wanted) { g=$9; s=$10; gsub(/^[[:space:]]+|[[:space:]]+$/, "", g); gsub(/^[[:space:]]+|[[:space:]]+$/, "", s); print g "\t" s }
    }' "$inventory"); then :; fi
[ -n "$goal" ] && [ -n "$step" ] || plan_err "work unit $unit not found in $inventory — nothing to remove (check the id)"

goal_file="$plan_dir/$goal/goal.md"
step_file="$plan_dir/$goal/steps/$step.md"
testing_file="$plan_dir/$goal/steps/$step-testing.md"
[ -f "$goal_file" ] || plan_err "goal file missing for $unit: $goal_file (goal '$goal' exists in the inventory but not on disk)"
[ -f "$step_file" ] || plan_err "step file missing for $unit: $step_file (rerun add-work-unit.sh to recreate it, then remove again)"

plan_prune_work_unit "$inventory" "$unit"
plan_rewrite_owned_work_units "$goal_file" "$inventory" "$goal"
rm -f "$step_file" "$testing_file"
plan_rebuild_goal_progress "$SCRIPT_DIR" "$plan_dir/$goal" "$goal"
plan_rebuild_plan_progress "$SCRIPT_DIR" "$plan_dir"

printf 'Removed work unit %s (%s).\n' "$unit" "$step"
printf '  reconciled: inventory, coverage, goal Owned work units, step files, progress\n'
