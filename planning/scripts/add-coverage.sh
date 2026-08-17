#!/usr/bin/env bash
set -euo pipefail

replace_mode=false
filtered_args=()
for arg in "$@"; do
    case "$arg" in
        --replace) replace_mode=true ;;
        *) filtered_args+=("$arg") ;;
    esac
done
set -- "${filtered_args[@]}"

if [ "$#" -ne 4 ]; then
    echo "Usage: $(basename "$0") <plan-directory> <required-outcome-or-proof> <WNN[,WNN...]> <notes> [--replace]" >&2
    exit 64
fi

plan_dir="$1"; outcome="$2"; work_units="$3"; notes="$4"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

plan_require_directory "$plan_dir"
for value_name in outcome work_units notes; do
    plan_require_safe_value "$value_name" "${!value_name}"
done
[[ "$work_units" =~ ^W[0-9][0-9]+(,[[:space:]]*W[0-9][0-9]+)*$ ]] || plan_die "Work units must be comma-separated IDs such as W01,W02"
inventory="$plan_dir/work-unit-inventory.md"
[ -f "$inventory" ] || plan_die "Work-unit inventory not found: $inventory"
plan_git_snapshot "$plan_dir"

temporary_file="${inventory}.tmp.$$"
trap 'rm -f "$temporary_file"' EXIT
if [ "$replace_mode" = true ]; then
    # Replace the coverage row whose first cell matches the outcome; create it
    # (before the Work units heading) when no such row exists yet.
    awk -F'|' -v row="| $outcome | $work_units | $notes |" -v wanted="$outcome" '
        function cell(v) { gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); gsub(/^`|`$/, "", v); return v }
        /^## Work units$/ && !inserted {
            if (!found) { print row; print "" }
            inserted = 1
        }
        /^\|/ && cell($2) == wanted {
            # Collapse every row with this outcome into one replacement at the
            # position of the first match; drop later duplicates.
            if (!found) { print row; found = 1 }
            next
        }
        { print }
        END { if (!inserted) exit 2 }
    ' "$inventory" > "$temporary_file" || plan_die "Inventory has no Work units section"
else
    awk -v row="| $outcome | $work_units | $notes |" '
        /^## Work units$/ && !inserted { print row; print ""; inserted = 1 }
        { print }
        END { if (!inserted) exit 2 }
    ' "$inventory" > "$temporary_file" || plan_die "Inventory has no Work units section"
fi
mv "$temporary_file" "$inventory"
trap - EXIT
if [ "$replace_mode" = true ]; then
    echo "Replaced (or added) coverage for $work_units"
else
    echo "Added coverage for $work_units"
fi
