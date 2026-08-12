#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "Usage: $(basename "$0") <plan-directory>" >&2
    exit 64
fi

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
trap - EXIT

echo "Created $inventory"
