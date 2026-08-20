#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: PROD
# create-work-unit-inventory.sh — seed a plan's work-unit-inventory.md with the
# coverage table, the work-unit table, and the decomposition-review checklist.
#
# create-plan.sh already emits an inventory (without the example rows), so this
# is the repair path for a plan whose inventory was lost: it refuses to overwrite
# an existing one (73). The example rows are placeholders the validator warns
# about until they are replaced by add-coverage.sh / add-work-unit.sh rows.
#
# Usage:
#   create-work-unit-inventory.sh [--plan-dir] <plan-directory>
#   create-work-unit-inventory.sh --help

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
inventory="$plan_dir/work-unit-inventory.md"

if [ ! -d "$plan_dir" ]; then
    echo "Plan directory not found: $plan_dir" >&2
    exit 66
fi
if [ -e "$inventory" ]; then
    echo "Work-unit inventory already exists: $inventory" >&2
    exit 73
fi

temporary_file="${inventory}.tmp.$$"
trap 'rm -f "$temporary_file"' EXIT
{
    printf '# Work-unit inventory: %s\n\n' "$(basename "$plan_dir")"
    printf '## Definition-of-done coverage\n\n'
    printf '| Required outcome or proof | Work unit IDs | Notes |\n'
    printf '|---|---|---|\n'
    printf '| <outcome> | W01 | <why this work unit covers it> |\n\n'
    printf '## Work units\n\n'
    printf '| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |\n'
    printf '|---|---|---|---|---|---|---|---|---|\n'
    printf '| W01 | source | `path/to/file` | `Class::method()` | `N/A` | <one concrete change> | — | 01-<goal> | 01-step-<slug> |\n\n'
    printf '## Decomposition review\n\n'
    printf '%s\n' '- [ ] Every definition-of-done item maps to one or more work units.'
    printf '%s\n' '- [ ] Every known affected file and changing symbol has its own work unit.'
    printf '%s\n' '- [ ] Every work unit has exactly one goal and one step.'
    printf '%s\n' '- [ ] Each goal has 2–10 work units, or records an allowed exception.'
    printf '%s\n' '- [ ] Each step has exactly one work unit and no unnamed incidental edits.'
    printf '%s\n' '- [ ] Dependencies form an executable order with no cycle.'
} > "$temporary_file"
mv "$temporary_file" "$inventory"

printf 'Created %s\n' "$inventory"
