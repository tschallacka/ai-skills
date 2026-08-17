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
step_descriptions=()
while IFS= read -r step_file; do
    step_name="$(basename "$step_file" .md)"
    step_names+=("$step_name")
    # Derive the row description from the step's Objective paragraph (§ 4.1),
    # so the tracker reflects the current step intent and survives rebuilds
    # without hand-filling (which the next rebuild would overwrite).
    description="$(awk '
        /^## Objective$/ { in_obj = 1; next }
        /^§ [0-9]+\.[0-9]+$/ && in_obj { after_label = 1; next }
        after_label && NF {
            line = $0; sub(/^[[:space:]]+/, "", line)
            if (length(line) > 100) line = substr(line, 1, 100) "..."
            print line; exit
        }
        /^## / && in_obj { exit }
    ' "$step_file")"
    [ -n "$description" ] || description="<short description>"
    step_descriptions+=("$description")
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
    for i in "${!step_names[@]}"; do
        printf '| %s | %s | %s | 💤 incomplete |\n' \
            "$goal_name" "${step_names[$i]}" "${step_descriptions[$i]}"
    done
} > "$temporary_file"
mv "$temporary_file" "$progress_file"
trap - EXIT

echo "Created $progress_file with ${#step_names[@]} step rows"
