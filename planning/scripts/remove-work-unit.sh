#!/usr/bin/env bash
# remove-work-unit.sh — remove a work unit and reconcile every reference to it.
#
# Removes, in one pass so no follow-up call is needed: the inventory row; the id
# from coverage rows (other ids kept; a row is dropped only when it empties);
# the goal's "Owned work units" section (re-derived); the step file and its
# -testing companion; and the goal + plan progress trackers.
#
# Usage:
#   remove-work-unit.sh <plan-directory> <WNN> [--confirm-cascade]
#   remove-work-unit.sh --help
#
# Note: the progress trackers are *rebuilt* from the step files, which resets
# completion statuses — re-apply them with update-step.sh afterwards.
#
# Refuses without --confirm-cascade when other work units list this one in
# their Depends-on column; the flag prunes those links (restore them on a
# re-add with `update-work-unit.sh --depends-on`).
#
# Exit codes: 64 bad invocation, unknown id, or a refused cascade; 66 the plan
# directory is missing.

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"
source "$script_dir/plan-reconcile-lib.sh"

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} <plan-directory> <WNN> [--confirm-cascade]
       ${0##*/} --help

Removes the inventory row, the id from coverage rows, the goal's Owned work
units section, the step file and its -testing companion, and rebuilds the goal
and plan progress trackers (which resets completion statuses — re-apply them
with update-step.sh).

Refuses without --confirm-cascade when other work units list this one in their
Depends-on column; the flag prunes those links.
USAGE
    exit "$rc"
}

# A flag loop, not a filtered_args pre-scan: on bash 3.2 expanding a
# possibly-empty "${array[@]}" is an unbound-variable abort under `set -u`, so
# a no-argument invocation would crash instead of printing usage.
plan_dir=""
unit=""
confirm_cascade=false
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --confirm-cascade) confirm_cascade=true; shift ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *)
            if [ -z "$plan_dir" ]; then
                plan_dir="$1"
            elif [ -z "$unit" ]; then
                unit="$1"
            else
                usage
            fi
            shift
            ;;
    esac
done
if [ -z "$plan_dir" ] || [ -z "$unit" ]; then
    usage
fi

plan_require_directory "$plan_dir"
plan_git_snapshot "$plan_dir"
[[ "$unit" =~ ^W[0-9][0-9]+$ ]] || plan_err "invalid work-unit id '$unit' — must be WNN (e.g. W01, W02)"
inventory="$plan_dir/work-unit-inventory.md"
[ -f "$inventory" ] || plan_err "work-unit inventory not found: $inventory (the plan appears incomplete)"

# Locate the inventory row.
goal=''; step=''
if plan_inventory_row "$inventory" "$unit"; then
    goal="$plan_inventory_goal"; step="$plan_inventory_step"
fi
[ -n "$goal" ] && [ -n "$step" ] || plan_err "work unit $unit not found in $inventory — nothing to remove (check the id)"

goal_file="$plan_dir/$goal/goal.md"
step_file="$plan_dir/$goal/steps/$step.md"
testing_file="$plan_dir/$goal/steps/$step-testing.md"
[ -f "$goal_file" ] || plan_err "goal file missing for $unit: $goal_file (goal '$goal' exists in the inventory but not on disk)"
[ -f "$step_file" ] || plan_err "step file missing for $unit: $step_file (rerun add-work-unit.sh to recreate it, then remove again)"

# Cascade guard: refuse when other units' Depends-on lists reference this unit,
# unless the caller accepted the cascade explicitly.
dependents=""; newline=$'\n'
while IFS= read -r row; do
    plan_inventory_split "$row"
    [ "$plan_inventory_id" != "$unit" ] || continue
    case ",${plan_inventory_depends// /}," in
        *",$unit,"*) dependents="${dependents:+$dependents$newline}$plan_inventory_id" ;;
    esac
done < <(plan_inventory_rows "$inventory")
if [ -n "$dependents" ]; then
    if [ "$confirm_cascade" = false ]; then
        plan_err "refusing to remove $unit: $(printf '%s' "$dependents" | tr '\n' ' ') still list it in Depends-on; rerun with --confirm-cascade to prune those links (and restore them after a re-add with update-work-unit.sh --depends-on)"
    fi
    printf 'plan: %s depends on %s; Depends-on links will be pruned\n' "$(printf '%s' "$dependents" | tr '\n' ' ')" "$unit" >&2
fi

plan_prune_work_unit "$inventory" "$unit"
plan_rewrite_owned_work_units "$goal_file" "$inventory" "$goal"
rm -f "$step_file" "$testing_file"
plan_rebuild_goal_progress "$script_dir" "$plan_dir/$goal" "$goal"
plan_rebuild_plan_progress "$script_dir" "$plan_dir"

# One line on stdout (§10). What was reconciled, and the update-step.sh
# follow-up, are stated in this file's docblock instead of nudged at runtime.
printf 'Removed work unit %s (%s)\n' "$unit" "$step"
