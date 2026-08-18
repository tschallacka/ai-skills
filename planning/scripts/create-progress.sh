#!/usr/bin/env bash
# create-progress.sh — generate a goal's progress.md: one row per implementation
# step, in step-name order, each carrying the step's own Objective text.
#
# Refuses to overwrite an existing tracker (73): rebuilding one is
# update-progress.sh's and rebuild-plan-progress.sh's job, not this script's.
# Testing companions (*-testing.md) are not steps and get no row.
#
# Usage:
#   create-progress.sh <goal-directory> <goal-name>
#   create-progress.sh --help

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} <goal-directory> <goal-name>
       ${0##*/} --help
USAGE
    exit "$rc"
}

case "${1:-}" in -h|--help) usage 0 ;; esac
[ "$#" -eq 2 ] || usage

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
    # Derive the description from the step's § 4.1 Objective: text hand-filled
    # here is lost on the next rebuild, and a generated table must never fall
    # back to a literal placeholder.
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

printf 'Created %s\n' "$progress_file"
