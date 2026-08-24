#!/usr/bin/env bash
# MODE: PROD
# remove-coverage.sh — remove one row from a plan's "## Definition-of-done
# coverage" table by its required-outcome cell. The sanctioned undo for
# add-coverage.sh (T17): an obsolete row whose work units still exist had no
# removal path short of the hand edit SKILL.md forbids.
#
# Per contract 9a the removal names what it discarded: each dropped row is
# printed on stderr after it is gone, with the work units it carried.
#
# Usage:
#   remove-coverage.sh [--plan-dir] <plan-directory> <required-outcome-or-proof>
#   remove-coverage.sh --help
#
# Exit codes: 64 bad invocation, 65 inventory or coverage section damaged,
# 66 plan directory, inventory, or matching row not found.

set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$script_dir/plan-document-lib.sh"
eval "set -- $(plan_hoist_plan_dir 1 "$@")"

export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--plan-dir] <plan-directory> <required-outcome-or-proof>
       ${0##*/} --help

Removes the coverage row whose Required outcome cell matches exactly.
USAGE
    exit "$rc"
}

plan_dir=""
outcome=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *)
            if [ -z "$plan_dir" ]; then
                plan_dir="$1"
            elif [ -z "$outcome" ]; then
                outcome="$1"
            else
                usage
            fi
            ;;
    esac
    shift
done
[ -n "$plan_dir" ] && [ -n "$outcome" ] || usage

plan_require_directory "$plan_dir"
plan_require_safe_value outcome "$outcome"
inventory="$plan_dir/work-unit-inventory.md"
[ -f "$inventory" ] || plan_die "Work-unit inventory not found: $inventory" 66
plan_git_snapshot "$plan_dir"

temporary_file="${inventory}.tmp.$$"
trap 'rm -f "$temporary_file"' EXIT

removed=0
awk -F'|' -v wanted="$outcome" '
    function cell(v) { gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); gsub(/^`|`$/, "", v); return v }
    BEGIN { in_coverage = 0 }
    /^## Definition-of-done coverage/ { in_coverage = 1; print; next }
    /^## / && in_coverage && $0 !~ /Definition-of-done coverage/ { in_coverage = 0; print; next }
    in_coverage && /^\|/ {
        header_or_sep = ($2 ~ /Required outcome/) || ($2 ~ /^-+[[:space:]]*-*/)
        if (!header_or_sep && cell($2) == wanted) {
            found = 1
            next
        }
    }
    { print }
    END { exit found ? 0 : 3 }
' "$inventory" > "$temporary_file" || removed=$?

if [ "$removed" -ne 0 ]; then
    rm -f "$temporary_file"
    plan_die "no coverage row with required outcome '$outcome' in $inventory (check the wording; add-coverage.sh lists rows via plan-content.sh)" 66
fi

# Contract 9a: name what the row carried, read from the original while the row
# is still there — a notice pointing at content that is already gone is worse
# than silence.
units="$(awk -F'|' -v wanted="$outcome" '
    function cell(v) { gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); gsub(/^`|`$/, "", v); return v }
    /^## Work units$/ { exit }
    /^\|/ && cell($2) == wanted { print cell($3) }
' "$inventory")"

mv "$temporary_file" "$inventory"
printf 'dropped coverage row for outcome %s: work units %s\n' "$outcome" "${units:-none}" >&2
printf 'Removed coverage for %s\n' "$outcome"
