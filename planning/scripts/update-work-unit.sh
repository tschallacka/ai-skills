#!/usr/bin/env bash
set -euo pipefail

# update-work-unit.sh — update the primary scope (and optionally the File) of a
# work-unit inventory row and its matching step file.
#
# Usage:
#   update-work-unit.sh <plan-directory> <WNN> <new-primary-scope> [<new-file>]
#
# The inventory row columns are: | ID | Type | File | Primary symbol or file
# scope | Subscope | Intended change | Depends on | Goal | Step |. The third
# positional updates the *Primary scope* (column $5); the optional fourth
# updates *File* (column $4). The step file's `- Primary symbol or file scope:`
# and `- File:` lines are updated to match.

if [ "$#" -ne 3 ] && [ "$#" -ne 4 ]; then
    printf 'Usage: %s <plan-directory> <WNN> <new-primary-scope> [<new-file>]\n' "$(basename "$0")" >&2
    exit 64
fi
plan_dir="$1" unit="$2" new_scope="$3" new_file="${4:-}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"
plan_require_directory "$plan_dir"
[[ "$unit" =~ ^W[0-9][0-9]+$ ]] || plan_die 'Work-unit ID must use WNN'
plan_require_safe_value new_scope "$new_scope"
if [ -n "$new_file" ]; then
    plan_require_safe_value new_file "$new_file"
fi
inventory="$plan_dir/work-unit-inventory.md"
[ -f "$inventory" ] || plan_die "Work-unit inventory not found: $inventory"
step_file="$(awk -F'|' -v wanted="$unit" '$0 ~ /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); if ($2 == wanted) {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $9); gsub(/^[[:space:]]+|[[:space:]]+$/, "", $10); print "'"$plan_dir"'/"$9"/steps/"$10".md"}}' "$inventory")"
[ -n "$step_file" ] && [ -f "$step_file" ] || plan_die "Work unit not found: $unit"
inventory_tmp="${inventory}.tmp.$$"; step_tmp="${step_file}.tmp.$$"
trap 'rm -f "$inventory_tmp" "$step_tmp"' EXIT
awk -F'|' -v wanted="$unit" -v replacement="$new_scope" -v newfile="$new_file" 'BEGIN{OFS="|"} /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {id=$2; gsub(/^[[:space:]]+|[[:space:]]+$/, "", id); if(id==wanted){$5=" " replacement " "; if(newfile != ""){$4=" " newfile " "}}} {print}' "$inventory" > "$inventory_tmp"
awk -v replacement="$new_scope" -v newfile="$new_file" '/^- Primary symbol or file scope:/ {print "- Primary symbol or file scope: " replacement; next} /^- File:/ {if (newfile != "") {print "- File: " newfile; next}} {print}' "$step_file" > "$step_tmp"
mv "$inventory_tmp" "$inventory"
mv "$step_tmp" "$step_file"
trap - EXIT
if [ -n "$new_file" ]; then
    printf 'Updated primary scope and File for %s\n' "$unit"
else
    printf 'Updated primary scope for %s\n' "$unit"
fi
