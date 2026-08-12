#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
    printf 'Usage: %s <plan-directory> <WNN>\n' "$(basename "$0")" >&2
    exit 64
fi
plan_dir="$1" unit="$2"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"
plan_require_directory "$plan_dir"
[[ "$unit" =~ ^W[0-9][0-9]+$ ]] || plan_die 'Work-unit ID must use WNN'
inventory="$plan_dir/work-unit-inventory.md"
[ -f "$inventory" ] || plan_die "Work-unit inventory not found: $inventory"
goal=''; step=''
if IFS=$'\t' read -r goal step < <(awk -F'|' -v wanted="$unit" '$0 ~ /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); if ($2 == wanted) {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $9); gsub(/^[[:space:]]+|[[:space:]]+$/, "", $10); print $9 "\t" $10}}' "$inventory"); then :; fi
if [ -n "${goal:-}" ] && [ -n "${step:-}" ]; then
    goal_file="$plan_dir/$goal/goal.md"
    step_file="$plan_dir/$goal/steps/$step.md"
    testing_file="$plan_dir/$goal/steps/$step-testing.md"
    [ -f "$goal_file" ] && [ -f "$step_file" ] || plan_die "Required work-unit files are missing: $unit"
    temporary_goal="${goal_file}.tmp.$$"
    trap 'rm -f "$temporary_goal"' EXIT
    awk -v unit="$unit" '{gsub("\\`" unit "\\` — [^.]*(\\. )?", ""); print}' "$goal_file" > "$temporary_goal"
    mv "$temporary_goal" "$goal_file"
    rm -f "$step_file" "$testing_file"
fi
temporary_inventory="${inventory}.tmp.$$"
trap 'rm -f "$temporary_inventory"' EXIT
awk -F'|' -v wanted="$unit" 'BEGIN{OFS="|"} /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {id=$2; gsub(/^[[:space:]]+|[[:space:]]+$/, "", id); if (id == wanted) next} /^\|/ {ids=$3; gsub(/^[[:space:]]+|[[:space:]]+$/, "", ids); n=split(ids, parts, ","); for (i=1; i<=n; i++) {gsub(/^[[:space:]]+|[[:space:]]+$/, "", parts[i]); if (parts[i] == wanted) next}} {print}' "$inventory" > "$temporary_inventory"
mv "$temporary_inventory" "$inventory"
trap - EXIT
printf 'Removed work unit %s\n' "$unit"
