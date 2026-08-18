#!/usr/bin/env bash
set -euo pipefail

# update-work-unit.sh — amend a work-unit inventory row and its matching step
# file in place.
#
# Usage:
#   update-work-unit.sh <plan-directory> <WNN> [<new-primary-scope>] [<new-file>]
#                       [--scope <text>] [--file <path>] [--type <type>]
#                       [--depends-on <WNN[,WNN...]|—>] [--description <text>]
#
# The inventory row columns are: | ID | Type | File | Primary symbol or file
# scope | Subscope | Intended change | Depends on | Goal | Step |. The third
# positional updates *Primary scope* (column 5); the optional fourth updates
# *File* (column 4). An empty positional leaves its column unchanged, so
# `"" "<path>"` updates File without touching scope. --scope/--file are the
# flag forms (equivalent to the positionals, for callers who prefer flags);
# flags update the remaining columns; nothing else changes, so coverage rows,
# the goal Owned work units section, and progress trackers are untouched —
# changing a dependency must never go through remove + re-add (that would drop
# the unit from its coverage rows and require manual repair).

usage() {
    printf 'Usage: %s <plan-directory> <WNN> [<new-primary-scope>] [<new-file>] [--scope <text>] [--file <path>] [--type <type>] [--depends-on <WNN[,WNN...]|—>] [--description <text>]\n' "$(basename "$0")" >&2
    exit 64
}

help() {
    printf 'Usage: %s <plan-directory> <WNN> [<new-primary-scope>] [<new-file>] [--scope <text>] [--file <path>] [--type <type>] [--depends-on <WNN[,WNN...]|—>] [--description <text>]\n' "$(basename "$0")"
    exit 0
}

case " $* " in
    *' --help '*|*' -h '*) help ;;
esac

[ "$#" -ge 2 ] || usage
plan_dir="$1" unit="$2"; shift 2
new_scope='' new_file='' new_type='' new_depends='' new_description=''
positional=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        --scope) [ "$#" -ge 2 ] || usage; new_scope="$2"; shift 2 ;;
        --file) [ "$#" -ge 2 ] || usage; new_file="$2"; shift 2 ;;
        --type) [ "$#" -ge 2 ] || usage; new_type="$2"; shift 2 ;;
        --depends-on) [ "$#" -ge 2 ] || usage; new_depends="$2"; shift 2 ;;
        --description) [ "$#" -ge 2 ] || usage; new_description="$2"; shift 2 ;;
        -h|--help) help ;;
        -*) usage ;;
        *)
            # Positional count, not -z: an empty positional means "leave
            # unchanged" and must not consume the next positional's slot.
            positional=$((positional + 1))
            if [ "$positional" -eq 1 ]; then
                new_scope="$1"
            elif [ "$positional" -eq 2 ]; then
                new_file="$1"
            else
                usage
            fi
            shift ;;
    esac
done
if [ -z "$new_scope" ] && [ -z "$new_file" ] && [ -z "$new_type" ] && [ -z "$new_depends" ] && [ -z "$new_description" ]; then
    usage
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"
plan_require_directory "$plan_dir"
plan_git_snapshot "$plan_dir"
[[ "$unit" =~ ^W[0-9][0-9]+$ ]] || plan_die 'Work-unit ID must use WNN'
for value_name in new_scope new_file new_type new_description; do
    if [ -n "${!value_name}" ]; then
        plan_require_safe_value "$value_name" "${!value_name}"
    fi
done
if [ -n "$new_depends" ]; then
    [[ "$new_depends" =~ ^(—|-)$ ]] || [[ "$new_depends" =~ ^W[0-9][0-9]+(,[[:space:]]*W[0-9][0-9]+)*$ ]] || plan_die "Depends-on must be a comma-separated WNN list, or —"
fi
inventory="$plan_dir/work-unit-inventory.md"
[ -f "$inventory" ] || plan_die "Work-unit inventory not found: $inventory"
step_file="$(awk -F'|' -v wanted="$unit" '$0 ~ /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); if ($2 == wanted) {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $9); gsub(/^[[:space:]]+|[[:space:]]+$/, "", $10); print "'"$plan_dir"'/"$9"/steps/"$10".md"}}' "$inventory")"
[ -n "$step_file" ] && [ -f "$step_file" ] || plan_die "Work unit not found: $unit"
inventory_tmp="${inventory}.tmp.$$"; step_tmp="${step_file}.tmp.$$"
trap 'rm -f "$inventory_tmp" "$step_tmp"' EXIT
awk -F'|' -v wanted="$unit" -v replacement="$new_scope" -v newfile="$new_file" -v newtype="$new_type" -v newdeps="$new_depends" 'BEGIN{OFS="|"} /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {id=$2; gsub(/^[[:space:]]+|[[:space:]]+$/, "", id); if(id==wanted){if(replacement != ""){$5=" " replacement " "}; if(newfile != ""){$4=" " newfile " "}; if(newtype != ""){$3=" " newtype " "}; if(newdeps != ""){$8=" " newdeps " "}}} {print}' "$inventory" > "$inventory_tmp"
awk -v replacement="$new_scope" -v newfile="$new_file" -v newtype="$new_type" '/^- Primary symbol or file scope:/ {if (replacement != "") {print "- Primary symbol or file scope: " replacement; next}} /^- File:/ {if (newfile != "") {print "- File: " newfile; next}} /^- Type:/ {if (newtype != "") {print "- Type: `" newtype "`"; next}} {print}' "$step_file" > "$step_tmp"
mv "$inventory_tmp" "$inventory"
mv "$step_tmp" "$step_file"
if [ -n "$new_description" ]; then
    plan_replace_paragraph "$step_file" '§ 4.1' "$new_description"
    awk -F'|' -v wanted="$unit" -v desc="$new_description" 'BEGIN{OFS="|"} /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {id=$2; gsub(/^[[:space:]]+|[[:space:]]+$/, "", id); if(id==wanted){$7=" " desc " "}} {print}' "$inventory" > "$inventory_tmp"
    mv "$inventory_tmp" "$inventory"
fi
trap - EXIT
changed=()
[ -n "$new_scope" ] && changed+=("scope")
[ -n "$new_file" ] && changed+=("file")
[ -n "$new_type" ] && changed+=("type")
[ -n "$new_depends" ] && changed+=("depends-on")
[ -n "$new_description" ] && changed+=("description")
printf 'Updated %s: %s\n' "$unit" "$(IFS=,; printf '%s' "${changed[*]}")"

# Changing a unit's behaviour (file, scope, or dependency edges) can invalidate
# the verification unit that grades it, even when that grader's own surfaces
# are unchanged. Surface the graders so the fixer re-reads them.
if [ -n "$new_file" ] || [ -n "$new_scope" ] || [ -n "$new_depends" ]; then
    graders="$(awk -F'|' -v wanted="$unit" '
        /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {
            id=$2; gsub(/^[[:space:]]+|[[:space:]]+$/, "", id)
            t=$3; gsub(/^[[:space:]]+|[[:space:]]+$/, "", t)
            if (t != "verification") next
            deps=$8; gsub(/^[[:space:]]+|[[:space:]]+$/, "", deps)
            n = split(deps, dp, ",")
            for (i = 1; i <= n; i++) { p = dp[i]; gsub(/^[[:space:]]+|[[:space:]]+$/, "", p); if (p == wanted) { print id; break } }
        }' "$inventory")"
    if [ -n "$graders" ]; then
        printf 'plan: %s changed behaviour; re-read its grader(s) %s — a grader checks the old behaviour until its own surfaces are updated\n' \
            "$unit" "$(printf '%s' "$graders" | tr '\n' ' ')"
    fi
fi