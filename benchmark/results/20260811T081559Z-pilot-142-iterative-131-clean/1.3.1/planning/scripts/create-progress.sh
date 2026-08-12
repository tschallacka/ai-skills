#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
    echo "Usage: $(basename "$0") <goal-directory> <goal-name>" >&2
    exit 64
fi

goal_dir="$1"
goal_name="$2"
steps_dir="$goal_dir/steps"
progress_file="$goal_dir/progress.md"

if [ ! -d "$steps_dir" ]; then
    echo "Steps directory not found: $steps_dir" >&2
    exit 66
fi
if [ -e "$progress_file" ]; then
    echo "Progress file already exists: $progress_file" >&2
    exit 73
fi

step_names=()
while IFS= read -r step_file; do
    step_names+=("$(basename "$step_file" .md)")
done < <(find "$steps_dir" -maxdepth 1 -type f -name '*.md' ! -name '*-testing.md' | sort)

if [ "${#step_names[@]}" -eq 0 ]; then
    echo "No step files found in: $steps_dir" >&2
    exit 66
fi

temporary_file="${progress_file}.tmp.$$"
trap 'rm -f "$temporary_file"' EXIT
{
    printf '# Progress: %s\n\n' "$goal_name"
    printf '**Progress:** `0%%  #### ----------------  100%%` 💤\n\n'
    printf '| Goalname | Stepname | Description | Completion status |\n'
    printf '|---|---|---|---|\n'
    for step_name in "${step_names[@]}"; do
        printf '| %s | %s | <short description> | 💤 incomplete |\n' \
            "$goal_name" "$step_name"
    done
} > "$temporary_file"
mv "$temporary_file" "$progress_file"
trap - EXIT

echo "Created $progress_file with ${#step_names[@]} step rows"
