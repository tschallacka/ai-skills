#!/usr/bin/env bash
# update-plan-progress.sh — set one goal's status row in the plan-level tracker
# and recompute the plan's overall bar.
#
# Rewrites the goal's row in <plan-directory>/progress.md (canonical 3 data
# columns: Goalname | Description | Completion status, so awk -F'|' reads the
# status from $4), then re-derives `**Overall progress:**` from every row. It
# refuses when the goal row is absent or appears more than once, because
# guessing which row to edit would silently corrupt the tracker.
#
# Usage:
#   update-plan-progress.sh <plan-directory> <goal-name> <incomplete|in-progress|completed>
#   update-plan-progress.sh --help
#
# Exit codes: 1 the goal row is not present exactly once, 64 bad invocation,
# 66 the plan has no progress.md.

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} <plan-directory> <goal-name> <incomplete|in-progress|completed>
       ${0##*/} --help
USAGE
    exit "$rc"
}

case "${1:-}" in
    -h|--help) usage 0 ;;
esac
[ "$#" -eq 3 ] || usage

plan_dir="$1"
goal_name="$2"
requested_status="$3"
progress_file="$plan_dir/progress.md"

# The glyphs are the on-disk contract; they must stay byte-identical.
case "$requested_status" in
    incomplete)
        status='💤 incomplete'
        ;;
    in-progress|in_progress)
        status='⏳ in progress'
        ;;
    completed)
        status='✅ completed'
        ;;
    *)
        printf 'Unknown status: %s\n' "$requested_status" >&2
        printf 'Use: incomplete, in-progress, or completed\n' >&2
        exit 64
        ;;
esac

[ -f "$progress_file" ] || plan_die "Progress file not found: $progress_file" 66
plan_git_snapshot "$plan_dir"

# One trap covers both temps. No `trap - EXIT` release: it discards the
# library's cleanup handler (§8), and dropping it after the first mv is what
# left the second write untrapped.
temporary_file="${progress_file}.tmp.$$"
trap 'rm -f "$temporary_file"' EXIT

awk -v wanted_goal="$goal_name" -v replacement="$status" '
    BEGIN { found = 0 }
    /^\|/ {
        goal = $2
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", goal)
        if (goal == wanted_goal) {
            sub(/\|[[:space:]]*[^|]*[[:space:]]*\|[[:space:]]*$/, "| " replacement " |")
            found++
        }
    }
    { print }
    END { if (found != 1) exit 1 }
' "$progress_file" > "$temporary_file" || {
    printf 'Goal row not found exactly once: %s\n' "$goal_name" >&2
    exit 1
}
mv "$temporary_file" "$progress_file"

read -r completed total < <(awk -F'|' '
    /^\|/ {
        goal = $2; status = $4
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", goal)
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", status)
        if (goal != "Goalname" && goal !~ /^-+$/ && status !~ /^-+$/) {
            total++
            if (status ~ /completed/) completed++
        }
    }
    END { print completed + 0, total + 0 }
' "$progress_file")

width=20
percent=0
if [ "$total" -gt 0 ]; then
    percent=$(( (completed * 100 + total / 2) / total ))
fi
filled=$(( percent * width / 100 )); empty=$(( width - filled ))
bar="$(printf '%*s' "$filled" '' | tr ' ' '#')$(printf '%*s' "$empty" '' | tr ' ' '-')"
icon='💤'; [ "$completed" -gt 0 ] && icon='⏳'; [ "$percent" -eq 100 ] && icon='✅'

sed "s|^\*\*Overall progress:\*\*.*$|**Overall progress:** \`${percent}%  ${bar}  100%\` ${icon}|" \
    "$progress_file" > "$temporary_file"
mv "$temporary_file" "$progress_file"

printf 'Updated %s (%s/%s goals, %s%%)\n' "$progress_file" "$completed" "$total" "$percent"
