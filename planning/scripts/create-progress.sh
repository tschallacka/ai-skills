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
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

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
    # without hand-filling (which the next rebuild would overwrite). Never fall
    # back to a literal placeholder — a generated table must not carry one.
    step_descriptions+=("$(plan_step_objective "$step_file" "$step_name")")
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
