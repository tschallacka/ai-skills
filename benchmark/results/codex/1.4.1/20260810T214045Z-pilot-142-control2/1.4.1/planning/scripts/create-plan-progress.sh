#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "Usage: $(basename "$0") <plan-directory>" >&2
    exit 64
fi

plan_dir="$1"
progress_file="$plan_dir/progress.md"

if [ -e "$progress_file" ]; then
    echo "Progress file already exists: $progress_file" >&2
    exit 73
fi

goal_names=()
while IFS= read -r goal_dir; do
    if [ -f "$goal_dir/goal.md" ]; then
        goal_names+=("$(basename "$goal_dir")")
    fi
done < <(find "$plan_dir" -mindepth 1 -maxdepth 1 -type d | sort)

if [ "${#goal_names[@]}" -eq 0 ]; then
    echo "No goal directories containing goal.md found in: $plan_dir" >&2
    exit 66
fi

temporary_file="${progress_file}.tmp.$$"
trap 'rm -f "$temporary_file"' EXIT
{
    printf '# Progress: %s\n\n' "$(basename "$plan_dir")"
    printf '**Overall progress:** `0%%  #### ----------------  100%%` 💤\n\n'
    printf '| Goalname | Description | Completion status |\n'
    printf '|---|---|---|\n'
    for goal_name in "${goal_names[@]}"; do
        printf '| %s | <short description> | 💤 incomplete |\n' "$goal_name"
    done
} > "$temporary_file"
mv "$temporary_file" "$progress_file"
trap - EXIT

echo "Created $progress_file with ${#goal_names[@]} goal rows"
