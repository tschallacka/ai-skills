#!/usr/bin/env bash
set -euo pipefail

case "${1:-}" in
    -h|--help)
        printf "Usage: %s <plan-directory>\n\nRebuilds the plan-level progress tracker from the goal's progress files.\n" "$(basename "$0")"
        exit 0
        ;;
esac
if [ "$#" -ne 1 ]; then
    printf 'Usage: %s <plan-directory>\n' "$(basename "$0")" >&2
    exit 64
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

plan_dir="$1"
[ -d "$plan_dir" ] || { printf 'Plan directory not found: %s\n' "$plan_dir" >&2; exit 66; }
progress_file="$plan_dir/progress.md"
[ -f "$progress_file" ] || { printf 'Plan progress file not found: %s\n' "$progress_file" >&2; exit 66; }
plan_git_snapshot "$plan_dir"

completed=0
total=0
temporary="${progress_file}.tmp.$$"
trap 'rm -f "$temporary"' EXIT
{
    printf '# Progress: %s\n\n' "$(basename "$plan_dir")"
    printf '**Overall progress:** `0%%  #### ----------------  100%%` 💤\n\n'
    printf '| Goalname | Description | Completion status |\n|---|---|---|\n'
    while IFS= read -r goal_dir; do
        [ -f "$goal_dir/goal.md" ] || continue
        goal_name="$(basename "$goal_dir")"
        goal_progress="$goal_dir/progress.md"
        status='💤 incomplete'
        if [ -f "$goal_progress" ] && grep -Fq '**Progress:** `100%' "$goal_progress"; then
            status='✅ completed'
            completed=$((completed + 1))
        elif [ -f "$goal_progress" ] && grep -Fq '⏳ in progress' "$goal_progress"; then
            status='⏳ in progress'
        fi
        total=$((total + 1))
        # Derive the Description from the goal's Outcome (Definition of done),
        # never a literal placeholder — a plan-level tracker with a hardcoded
        # <short description> carries no information.
        desc="$(plan_goal_definition_of_done "$goal_dir/goal.md" "$goal_name")"
        printf '| %s | %s | %s |\n' "$goal_name" "$desc" "$status"
    done < <(find "$plan_dir" -mindepth 1 -maxdepth 1 -type d | sort)
    [ "$total" -gt 0 ] || { printf 'No goal directories found\n' >&2; exit 66; }
} > "$temporary"
percent=$(( (completed * 100 + total / 2) / total ))
filled=$(( percent * 20 / 100 )); empty=$(( 20 - filled ))
bar="$(printf '%*s' "$filled" '' | tr ' ' '#')$(printf '%*s' "$empty" '' | tr ' ' '-')"
icon='💤'; [ "$completed" -gt 0 ] && icon='⏳'; [ "$percent" -eq 100 ] && icon='✅'
sed -i "s|^\*\*Overall progress:\*\*.*$|**Overall progress:** \`${percent}%  ${bar}  100%\` ${icon}|" "$temporary"
mv "$temporary" "$progress_file"
trap - EXIT
printf '%s: %s/%s goals, %s%%\n' "$(basename "$plan_dir")" "$completed" "$total" "$percent"
