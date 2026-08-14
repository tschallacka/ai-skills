#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 3 ]; then
    echo "Usage: $(basename "$0") <plan-directory> <goal-name> <incomplete|in-progress|completed>" >&2
    exit 64
fi

plan_dir="$1"
goal_name="$2"
requested_status="$3"
progress_file="$plan_dir/progress.md"

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
        echo "Unknown status: $requested_status" >&2
        echo "Use: incomplete, in-progress, or completed" >&2
        exit 64
        ;;
esac

if [ ! -f "$progress_file" ]; then
    echo "Progress file not found: $progress_file" >&2
    exit 66
fi

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
    echo "Goal row not found exactly once: $goal_name" >&2
    exit 1
}
mv "$temporary_file" "$progress_file"
trap - EXIT

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

temporary_file="${progress_file}.tmp.$$"
sed "s|^\*\*Overall progress:\*\*.*$|**Overall progress:** \`${percent}%  ${bar}  100%\` ${icon}|" \
    "$progress_file" > "$temporary_file"
mv "$temporary_file" "$progress_file"
trap - EXIT

echo "$(basename "$plan_dir"): ${completed}/${total} goals, ${percent}%"
