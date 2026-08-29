#!/usr/bin/env bash
# MODE: PROD
# add-coverage.sh — add (or with --replace, replace) one row in a plan's
# "## Definition-of-done coverage" table, linking a required outcome or proof to
# the work units that deliver it.
#
# The row is inserted immediately above the "## Work units" heading, so coverage
# rows accumulate in the order they were added. --replace collapses every row
# carrying the same outcome into one at the position of the first match, and adds
# the row when no such outcome exists yet.
#
# Usage:
#   add-coverage.sh [--plan-dir] <plan-directory> <required-outcome-or-proof> <WNN[,WNN...]> <notes> [--replace]
#   add-coverage.sh --help

set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$script_dir/plan-document-lib.sh"
# Accept --plan-dir as a synonym for the positional plan directory: the
# bounded reader takes the flag, so a reader who learned it there is not
# refused here.
eval "set -- $(plan_hoist_plan_dir 1 "$@")"

export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--plan-dir] <plan-directory> <required-outcome-or-proof> <WNN[,WNN...]> <notes> [--replace]
       ${0##*/} --help
USAGE
    exit "$rc"
}

# A flag loop, not a "$@" pre-scan into an array: -h belongs in band, and
# re-setting "$@" from a possibly-empty array aborts under set -u on bash 3.2.
replace_mode=false
positional=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --replace) replace_mode=true; shift ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *) positional+=("$1"); shift ;;
    esac
done
while [ "$#" -gt 0 ]; do positional+=("$1"); shift; done

set -- ${positional[@]+"${positional[@]}"}
[ "$#" -eq 4 ] || usage
plan_dir="$1"; outcome="$2"; work_units="$3"; notes="$4"

plan_require_directory "$plan_dir"
for value_name in outcome work_units notes; do
    plan_require_safe_value "$value_name" "${!value_name}"
done
[[ "$work_units" =~ ^W[0-9][0-9]+(,[[:space:]]*W[0-9][0-9]+)*$ ]] || plan_die "Work units must be comma-separated IDs such as W01,W02"
inventory="$plan_dir/work-unit-inventory.md"
if [ ! -f "$inventory" ]; then
    printf '%s: %s\n' "${0##*/}" "Work-unit inventory not found: $inventory" >&2
    exit 66
fi
plan_git_snapshot "$plan_dir"

temporary_file="${inventory}.tmp.$$"
trap 'rm -f "$temporary_file"' EXIT
# A --replace that matches nothing is an insert, not a replacement, and the usual
# cause is that the outcome -- the match key -- was reworded in the same call that
# corrected the ids. Deciding the verb before the write lets the caller be told,
# rather than reading "Replaced" beside two rows disagreeing about one outcome (T68).
replaced_existing=false
if [ "$replace_mode" = true ]; then
    while IFS= read -r existing_row; do
        case "$existing_row" in '|'*) ;; *) continue ;; esac
        [ "$(plan_table_cell "$existing_row" 2)" = "$outcome" ] || continue
        replaced_existing=true
        break
    done < "$inventory"
fi

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
            # position of the first match; drop later duplicates. Each dropped
            # row is named on stderr: it carried work units a person chose, and
            # collapsing three rows into one while reporting only "Replaced"
            # loses that silently.
            if (!found) { print row; found = 1 }
            else { printf "dropped duplicate coverage row for outcome %s: work units %s\n", wanted, cell($3) > "/dev/stderr" }
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
record_id="$(awk -v wanted="$outcome" '
    BEGIN { FS = "|" }
    function cell(v) { gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); return v }
    /^## Work units$/ { exit }
    /^\|/ && $2 !~ /Required outcome/ && $2 !~ /^-+$/ {
        row++
        if (cell($2) == wanted && found == 0) found = row
    }
    END { if (found > 0) printf "coverage:%02d\n", found; else exit 1 }
' "$inventory")" || plan_die "Coverage row was written but could not be located"
if [ "$replace_mode" = true ]; then
    if [ "$replaced_existing" = true ]; then
        printf 'Replaced coverage for %s (%s)\n' "$work_units" "$record_id"
    else
        printf 'Added coverage for %s (%s) -- --replace matched no existing outcome, so this is a new row, not a replacement. If you meant to replace one, its wording differs; look for a stale row under the old outcome.\n' \
            "$work_units" "$record_id"
    fi
else
    printf 'Added coverage for %s (%s)\n' "$work_units" "$record_id"
fi
