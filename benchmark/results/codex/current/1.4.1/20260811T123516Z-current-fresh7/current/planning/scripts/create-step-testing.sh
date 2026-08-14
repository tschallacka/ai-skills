#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 3 ]; then
    printf 'Usage: %s <goal-directory> <step-name> <verification-instructions>\n' "$(basename "$0")" >&2
    exit 64
fi

goal_dir="$1"
step_name="$2"
instructions="$3"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

plan_require_directory "$goal_dir"
[[ "$step_name" =~ ^[0-9][0-9]-step-[a-z0-9-]+$ ]] || plan_die "Step name must use 01-step-kebab-case"
plan_require_safe_value instructions "$instructions"
step_file="$goal_dir/steps/$step_name.md"
[ -f "$step_file" ] || plan_die "Implementation step not found: $step_file"
testing_file="$goal_dir/steps/${step_name}-testing.md"
[ ! -e "$testing_file" ] || plan_die "Testing companion already exists: $testing_file"

temporary_file="${testing_file}.tmp.$$"
trap 'rm -f "$temporary_file"' EXIT
{
    printf '# Verification: %s\n\n' "$step_name"
    printf '## Automated tests\n\n%s\n' "$instructions"
} > "$temporary_file"
mv "$temporary_file" "$testing_file"
trap - EXIT
printf 'Created testing companion %s\n' "$testing_file"
