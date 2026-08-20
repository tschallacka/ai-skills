#!/usr/bin/env bash
# MODE: PROD
# add-ui-story-links.sh — update one UI story's related work-unit references.
#
# Usage:
#   add-ui-story-links.sh [--plan-dir] <plan-directory> <US-NN> <WNN[,WNN...]>
#   add-ui-story-links.sh --help

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"
eval "set -- $(plan_hoist_plan_dir 1 "$@")"

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--plan-dir] <plan-directory> <US-NN> <WNN[,WNN...]>
       ${0##*/} --help
USAGE
    exit "$rc"
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *) break ;;
    esac
done
[ "$#" -eq 3 ] || usage

plan_dir="$1"
story_id="$2"
work_units="$3"
plan_require_directory "$plan_dir"
[[ "$story_id" =~ ^US-[0-9][0-9]+$ ]] || plan_die "Story ID must use US-01"
[[ "$work_units" =~ ^W[0-9][0-9]+(,[[:space:]]*W[0-9][0-9]+)*$ ]] || plan_die "Work units must be comma-separated IDs such as W01,W02"

stories="$plan_dir/ui-user-stories.md"
inventory="$plan_dir/work-unit-inventory.md"
[ -f "$stories" ] || plan_die "UI story artifact not found; run create-ui-validation.sh first" 66
[ -f "$inventory" ] || plan_die "Work-unit inventory not found: $inventory" 66

printf '%s\n' "$work_units" | tr ',' '\n' | while IFS= read -r unit; do
    unit="$(printf '%s' "$unit" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
    plan_inventory_row "$inventory" "$unit" >/dev/null || plan_die "Related work unit not found: $unit" 66
done

if ! awk -v wanted="$story_id" 'BEGIN { FS = "|" } function t(v){gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); return v} /^\|/ && t($2)==wanted {found=1} END {exit !found}' "$stories"; then
    plan_die "Story ID not found: $story_id" 66
fi

plan_git_snapshot "$plan_dir"
awk -v wanted="$story_id" -v units="$work_units" '
    BEGIN { FS = "|"; OFS = "|" }
    function t(v) { gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); return v }
    /^\|/ && t($2) == wanted {
        $9 = " " units " "
        touched = 1
    }
    { print }
    END { if (!touched) exit 2 }
' "$stories" | plan_atomic_write "$stories" || plan_die "UI story table row not found"
printf 'Updated %s related_work_units=%s\n' "$story_id" "$work_units"
