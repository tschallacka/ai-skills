#!/usr/bin/env bash
# MODE: PROD
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
#   rebuild-plan-progress.sh [--plan-dir] <plan-directory>
#   rebuild-plan-progress.sh --help
#
# Exit codes: 64 bad invocation, 66 the plan directory, its progress.md, or any
# goal directory is missing.

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
rows=''
# Scan first, then write once: the bar needs the totals, and composing the file
# in a single pass removes the placeholder-plus-sed substitution this used to
# need (T5).
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
    # The newline is appended outside the substitution: $( ) strips it, and an
    # interior loss would concatenate this row onto the previous one.
    rows="${rows}$(printf '| %s | %s | %s |' "$goal_name" "$desc" "$status")"$'\n'
done < <(find "$plan_dir" -mindepth 1 -maxdepth 1 -type d | sort)
[ "$total" -gt 0 ] || { printf 'No goal directories found\n' >&2; exit 66; }

# The glyphs are an on-disk contract — test-progress-bar-shape.sh pins them
# byte for byte — so both the percentage and the bar come from the canonical
# helpers instead of a local re-derivation that can drift. The width is the
# documented default, named here so it is not a bare literal.
bar_width=20
percent="$(plan_progress_percent "$completed" "$total")"
bar="$(plan_progress_bar "$completed" "$total" "$bar_width")"
icon='💤'; [ "$completed" -gt 0 ] && icon='⏳'; [ "$percent" -eq 100 ] && icon='✅'

temporary="${progress_file}.tmp.$$"
trap 'rm -f "$temporary"' EXIT
{
    printf '# Progress: %s\n\n' "$plan_name"
    printf '**Overall progress:** `%s%%  %s  100%%` %s\n\n' "$percent" "$bar" "$icon"
    printf '| Goalname | Description | Completion status |\n|---|---|---|\n'
    printf '%s' "$rows"
} > "$temporary"
mv "$temporary" "$progress_file"

printf 'Updated %s (%s/%s goals, %s%%)\n' "$progress_file" "$completed" "$total" "$percent"
