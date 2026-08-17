#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "Usage: $(basename "$0") <goal-directory>" >&2
    exit 64
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

goal_dir="$1"
progress_file="$goal_dir/progress.md"

if [ ! -f "$progress_file" ]; then
    echo "Progress file not found: $progress_file" >&2
    exit 66
fi
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
trap - EXIT

echo "$(basename "$goal_dir"): ${completed}/${total} steps, ${percent}%"
