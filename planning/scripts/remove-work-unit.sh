#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/plan-document-lib.sh"
source "$SCRIPT_DIR/plan-reconcile-lib.sh"

HELP=$'Removes a work unit and reconciles every reference so you do not need to\nfollow up by hand.\n\nUsage: remove-work-unit.sh <plan-directory> <WNN> [--confirm-cascade]\n\nRemoves: the inventory row; the id from coverage rows (others kept; row dropped\nonly if empty); the goal Owned work units section (re-derived); the step file\nand its -testing companion; and the goal + plan progress trackers (rebuilt,\nwhich resets completion statuses — re-apply them with update-step.sh).\n\nRefuses without --confirm-cascade when other work units list this one in their\nDepends-on column; the flag prunes those links (restore them on a re-add with\nupdate-work-unit.sh --depends-on).'
confirm_cascade=false
filtered_args=()
for arg in "$@"; do
    if [ "$arg" = --confirm-cascade ]; then
        confirm_cascade=true
    else
        filtered_args+=("$arg")
    fi
done
plan_guard 2 "$HELP" "${filtered_args[@]}"
plan_dir="${filtered_args[0]}"; unit="${filtered_args[1]}"

plan_require_directory "$plan_dir" || plan_err "plan directory not found: $plan_dir (pass an absolute path, e.g. .plans/<plan-name>)"
plan_git_snapshot "$plan_dir"
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

# Cascade guard: refuse when other units' Depends-on lists reference this unit,
# unless the caller accepted the cascade explicitly.
dependents="$(awk -F'|' -v wanted="$unit" '
    /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {
        id=$2; gsub(/^[[:space:]]+|[[:space:]]+$/, "", id)
        if (id == wanted) next
        deps=$8; gsub(/^[[:space:]]+|[[:space:]]+$/, "", deps)
        n = split(deps, dp, ",")
        for (i = 1; i <= n; i++) { p = dp[i]; gsub(/^[[:space:]]+|[[:space:]]+$/, "", p); if (p == wanted) { print id; break } }
    }' "$inventory")"
if [ -n "$dependents" ]; then
    if [ "$confirm_cascade" = false ]; then
        plan_err "refusing to remove $unit: $(printf '%s' "$dependents" | tr '\n' ' ') still list it in Depends-on; rerun with --confirm-cascade to prune those links (and restore them after a re-add with update-work-unit.sh --depends-on)"
    fi
    printf 'plan: %s depends on %s; Depends-on links will be pruned\n' "$(printf '%s' "$dependents" | tr '\n' ' ')" "$unit" >&2
fi

plan_prune_work_unit "$inventory" "$unit"
plan_rewrite_owned_work_units "$goal_file" "$inventory" "$goal"
rm -f "$step_file" "$testing_file"
plan_rebuild_goal_progress "$SCRIPT_DIR" "$plan_dir/$goal" "$goal"
plan_rebuild_plan_progress "$SCRIPT_DIR" "$plan_dir"

printf 'Removed work unit %s (%s).\n' "$unit" "$step"
printf '  reconciled: inventory, coverage, goal Owned work units, step files, progress\n'
printf '  note: goal progress was rebuilt from step files; re-apply completion statuses with update-step.sh\n'
