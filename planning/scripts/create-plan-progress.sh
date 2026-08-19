#!/usr/bin/env bash
# create-plan-progress.sh — generate a plan's top-level progress.md: one row per
# goal directory holding a goal.md, each carrying that goal's definition of done.
#
# Row order is the goal directories in byte order (export LC_ALL=C above, so the
# generated order is the same for every developer on every locale). Refuses to
# overwrite an existing tracker (73); rebuild-plan-progress.sh refreshes one.
#
# Usage:
#   create-plan-progress.sh [--plan-dir] <plan-directory>
#   create-plan-progress.sh --help

set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$script_dir/plan-document-lib.sh"
# Accept --plan-dir as a synonym for the positional plan directory: the
# bounded reader takes the flag, so a reader who learned it there is not
# refused here.
eval "set -- $(plan_hoist_plan_dir 1 "$@")"

export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--plan-dir] <plan-directory>
       ${0##*/} --help
USAGE
    exit "$rc"
}

case "${1:-}" in -h|--help) usage 0 ;; esac
[ "$#" -eq 1 ] || usage

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
        desc="$(plan_goal_definition_of_done "$plan_dir/$goal_name/goal.md" "$goal_name")"
        printf '| %s | %s | 💤 incomplete |\n' "$goal_name" "$desc"
    done
} > "$temporary_file"
mv "$temporary_file" "$progress_file"

printf 'Created %s\n' "$progress_file"
