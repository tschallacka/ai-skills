#!/usr/bin/env bash
# rebuild-plan-progress.sh — regenerate a plan's progress tracker from its
# goals' own progress files.
#
# Discards the plan-level table and rebuilds it: one row per goal directory that
# holds a goal.md, its Description derived from the goal's Outcome (never a
# literal placeholder), and its status read back from the goal's progress.md
# (`**Progress:** \`100%` means completed, a `⏳ in progress` cell means in
# progress). It resets completion statuses that were set by hand, so callers
# re-apply them with update-step.sh afterwards.
#
# Usage:
#   rebuild-plan-progress.sh <plan-directory>
#   rebuild-plan-progress.sh --help
#
# Exit codes: 64 bad invocation, 66 the plan directory, its progress.md, or any
# goal directory is missing.

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} <plan-directory>
       ${0##*/} --help

Rebuilds the plan-level progress tracker from the goals' progress files.
USAGE
    exit "$rc"
}

plan_dir=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *) [ -z "$plan_dir" ] || usage; plan_dir="$1"; shift ;;
    esac
done
[ -n "$plan_dir" ] || usage

plan_require_directory "$plan_dir"
progress_file="$plan_dir/progress.md"
[ -f "$progress_file" ] || plan_die "Plan progress file not found: $progress_file" 66
plan_git_snapshot "$plan_dir"

plan_name="$(basename "$plan_dir")"
completed=0
total=0
temporary="${progress_file}.tmp.$$"
barred="${progress_file}.bar.$$"
trap 'rm -f "$temporary" "$barred"' EXIT
{
    printf '# Progress: %s\n\n' "$plan_name"
    # Placeholder only: the real bar is substituted below before the file is
    # moved into place. Glyphs are pinned by test-progress-bar-shape.sh.
    printf '**Overall progress:** `0%%  #### ----------------  100%%` 💤\n\n'
    printf '| Goalname | Description | Completion status |\n|---|---|---|\n'
    # LC_ALL=C (exported above) pins the collation: a bare `sort` orders the
    # generated rows by the ambient locale, so two developers would otherwise
    # generate different row orders from the same plan.
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

# Drops the total>0 guard, which is safe only because the block above already
# exited 66 when total is 0.
percent=$(( (completed * 100 + total / 2) / total ))
filled=$(( percent * 20 / 100 )); empty=$(( 20 - filled ))
bar="$(printf '%*s' "$filled" '' | tr ' ' '#')$(printf '%*s' "$empty" '' | tr ' ' '-')"
icon='💤'; [ "$completed" -gt 0 ] && icon='⏳'; [ "$percent" -eq 100 ] && icon='✅'
sed "s|^\*\*Overall progress:\*\*.*$|**Overall progress:** \`${percent}%  ${bar}  100%\` ${icon}|" \
    "$temporary" > "$barred"
mv "$barred" "$progress_file"

printf 'Updated %s (%s/%s goals, %s%%)\n' "$progress_file" "$completed" "$total" "$percent"
