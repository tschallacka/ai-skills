#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: PROD
# add-goal.sh — create one goal directory (goal.md plus an empty steps/) in a
# plan and refresh the plan-level progress tracker.
#
# The goal's own progress.md is deliberately NOT created here: create-progress.sh
# needs step files to exist, so add-work-unit.sh creates it with the goal's first
# work unit. Fill the emitted placeholders with update-plan-content.sh; the
# Goal-size exception heading is emitted empty on purpose (see below).
#
# Usage:
#   add-goal.sh [--plan-dir] <plan-directory> <goal-name> <title> <outcome>
#   add-goal.sh --help

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
Usage: ${0##*/} [--plan-dir] <plan-directory> <goal-name> <title> <outcome>
       ${0##*/} --help
USAGE
    exit "$rc"
}

case "${1:-}" in -h|--help) usage 0 ;; esac
[ "$#" -eq 4 ] || usage

plan_dir="$1"
goal_name="$2"
title="$3"
outcome="$4"

plan_require_directory "$plan_dir"
plan_git_snapshot "$plan_dir"
[[ "$goal_name" =~ ^[0-9][0-9]-[a-z0-9-]+$ ]] || plan_die "Goal name must use 01-kebab-case"
plan_require_safe_value title "$title"
plan_require_safe_value outcome "$outcome"
goal_dir="$plan_dir/$goal_name"
if [ -e "$goal_dir" ]; then
    printf '%s: %s\n' "${0##*/}" "Goal already exists: $goal_dir" >&2
    exit 73
fi

mkdir -p "$goal_dir/steps"
goal_file="$goal_dir/goal.md"
# The trap also removes the directories this script created, not just the temp.
temporary_file="${goal_file}.tmp.$$"
trap 'rm -f "$temporary_file"; rmdir "$goal_dir/steps" "$goal_dir" 2>/dev/null || true' EXIT
{
    printf '# Goal: %s\n\n' "$title"
    printf '## Current state and prior-goal handoffs\n\n§ 2.1\n<confirmed facts and prerequisite handoffs>\n\n'
    printf '## Outcome and definition of done\n\n§ 3.1\n%s\n\n' "$outcome"
    printf '## Why this goal is needed\n\n§ 4.1\n<how this goal contributes to the initiative>\n\n'
    printf '## Scope\n\n§ 5.1\n<included and explicitly excluded behavior>\n\n'
    printf '## Affected files, systems, data, and interfaces\n\n§ 6.1\n<concrete affected areas>\n\n'
    printf '## Dependencies and handoffs\n\n§ 7.1\n<prerequisites and precise downstream handoffs>\n\n'
    printf '## Implementation approach, risks, and edge cases\n\n§ 8.1\n<approach, risks, and edge cases>\n\n'
    printf '## Owned work units\n\n§ 9.1\n<add work units with add-work-unit.sh>\n\n'
    printf '## Testing requirement\n\n'
    printf '| Test required | Rationale |\n'
    printf '|---|---|\n'
    printf '| no | <set to yes when this goal has a testable behavior; explain research or other untestable goals> |\n\n'
    # Emitted empty on purpose: a placeholder is valid to write and invalid to
    # keep. The validator requires content here once the goal owns a single
    # work unit.
    printf '## Goal-size exception\n'
} > "$temporary_file"
mv "$temporary_file" "$goal_file"
trap - EXIT
# No per-goal tracker here: generating one needs step files, which do not exist
# until the goal's first work unit lands.
if [ ! -f "$plan_dir/progress.md" ]; then
    "$script_dir/create-plan-progress.sh" "$plan_dir" >/dev/null
fi
"$script_dir/rebuild-plan-progress.sh" "$plan_dir" >/dev/null
printf 'Created %s\n' "$goal_dir"
