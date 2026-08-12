#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 4 ]; then
    echo "Usage: $(basename "$0") <plan-directory> <goal-name> <title> <outcome>" >&2
    exit 64
fi

plan_dir="$1"
goal_name="$2"
title="$3"
outcome="$4"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

plan_require_directory "$plan_dir"
[[ "$goal_name" =~ ^[0-9][0-9]-[a-z0-9-]+$ ]] || plan_die "Goal name must use 01-kebab-case"
plan_require_safe_value title "$title"
plan_require_safe_value outcome "$outcome"
goal_dir="$plan_dir/$goal_name"
[ ! -e "$goal_dir" ] || plan_die "Goal already exists: $goal_dir"

mkdir -p "$goal_dir/steps"
goal_file="$goal_dir/goal.md"
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
    printf '## Goal-size exception\n\n§ 11.1\n<required only when this goal has one permitted work unit>\n'
} > "$temporary_file"
mv "$temporary_file" "$goal_file"
trap - EXIT
echo "Created $goal_dir"
