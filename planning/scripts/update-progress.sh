#!/usr/bin/env bash
# MODE: PROD
# update-progress.sh — recompute a goal's progress bar from its own step rows.
#
# Reads the goal's progress.md step table (canonical 4 data columns: Goalname |
# Stepname | Description | Completion status, so awk -F'|' sees the status in
# $5) and rewrites the single `**Progress:**` line with the percentage, a
# 20-cell bar, and a status icon. It never touches step rows — update-step.sh
# owns those, and calls this script afterwards.
#
# Usage:
#   update-progress.sh <goal-directory>
#   update-progress.sh --help
#
# Exit codes: 64 bad invocation, 66 the goal has no progress.md.

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} <goal-directory>
       ${0##*/} --help
USAGE
    exit "$rc"
}

goal_dir=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *) [ -z "$goal_dir" ] || usage; goal_dir="$1"; shift ;;
    esac
done
[ -n "$goal_dir" ] || usage

progress_file="$goal_dir/progress.md"
[ -f "$progress_file" ] || plan_die "Progress file not found: $progress_file" 66
plan_git_snapshot "$(dirname "$goal_dir")"

read -r completed total < <(awk -F'|' '
    /^\|/ {
        goal = $2; status = $5
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", goal)
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", status)
        if (goal != "Goalname" && goal !~ /^-+$/ && status !~ /^-+$/) {
            total++
            if (status ~ /completed/) completed++
        }
    }
    END { print completed + 0, total + 0 }
' "$progress_file")

# Canonical percent/bar/icon derivation. Half-up rounding (+ total / 2) and the
# 20-column width are the on-disk contract.
width=20
percent=0
if [ "$total" -gt 0 ]; then
    percent=$(( (completed * 100 + total / 2) / total ))
fi

filled=$(( percent * width / 100 ))
empty=$(( width - filled ))
bar="$(printf '%*s' "$filled" '' | tr ' ' '#')$(printf '%*s' "$empty" '' | tr ' ' '-')"

icon='💤'
if [ "$completed" -gt 0 ]; then
    icon='⏳'
fi
if [ "$percent" -eq 100 ]; then
    icon='✅'
fi

temporary_file="${progress_file}.tmp.$$"
trap 'rm -f "$temporary_file"' EXIT
sed "s|^\*\*Progress:\*\*.*$|**Progress:** \`${percent}%  ${bar}  100%\` ${icon}|" \
    "$progress_file" > "$temporary_file"
mv "$temporary_file" "$progress_file"

printf 'Updated %s (%s/%s steps, %s%%)\n' "$progress_file" "$completed" "$total" "$percent"
