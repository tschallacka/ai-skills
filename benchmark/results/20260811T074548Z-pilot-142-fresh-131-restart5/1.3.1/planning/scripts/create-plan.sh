#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
    echo "Usage: $(basename "$0") <plan-directory> <title>" >&2
    exit 64
fi

plan_dir="$1"
title="$2"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

[[ "$(basename "$plan_dir")" =~ ^[a-z0-9][a-z0-9-]*$ ]] || plan_die "Plan directory name must be kebab-case"
[ ! -e "$plan_dir" ] || plan_die "Plan directory already exists: $plan_dir"
plan_require_safe_value title "$title"

mkdir -p "$plan_dir"
description="$plan_dir/plan-description.md"
temporary_file="${description}.tmp.$$"
trap 'rm -f "$temporary_file"; rmdir "$plan_dir" 2>/dev/null || true' EXIT
{
    printf '# Plan: %s\n\n' "$title"
    printf '## Current state\n\n§ 2.1\n<confirmed facts, available assets, and relevant prior work>\n\n'
    printf '## Desired outcome\n\n§ 3.1\n<definition of done>\n\n'
    printf '## Approach\n\n§ 4.1\n<agreed sequence and major implementation decisions>\n\n'
    printf '## Scope\n\n§ 5.1\n<included and explicitly excluded behavior>\n\n'
    printf '## Affected areas\n\n§ 6.1\n<files, modules, layouts, services, data, and systems>\n\n'
    printf '## Constraints and decisions\n\n§ 7.1\n<permissions, ownership, conventions, and user choices>\n\n'
    printf '## Risks and open questions\n\n§ 8.1\n<items that could affect execution>\n\n'
    printf '## UI classification\n\n- UI affected: no\n- Rationale: <why>\n\n'
    printf '## Adversarial review\n\n- Artifact: `adversarial-review.md`\n- Status: 💤 pending\n'
} > "$temporary_file"
mv "$temporary_file" "$description"
inventory="$plan_dir/work-unit-inventory.md"
{
    printf '# Work-unit inventory: %s\n\n' "$(basename "$plan_dir")"
    printf '## Definition-of-done coverage\n\n'
    printf '| Required outcome or proof | Work unit IDs | Notes |\n|---|---|---|\n\n'
    printf '## Work units\n\n'
    printf '| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |\n'
    printf '|---|---|---|---|---|---|---|---|---|\n\n'
    printf '## Decomposition review\n\n'
    printf '%s\n' '- [ ] Every definition-of-done item maps to one or more work units.'
    printf '%s\n' '- [ ] Every known affected file and changing symbol has its own work unit.'
    printf '%s\n' '- [ ] Every work unit has exactly one goal and one step.'
    printf '%s\n' '- [ ] Each goal has 2–10 work units, or records an allowed exception.'
    printf '%s\n' '- [ ] Each step has exactly one work unit and no unnamed incidental edits.'
    printf '%s\n' '- [ ] Dependencies form an executable order with no cycle.'
} > "$inventory"
trap - EXIT
echo "Created $plan_dir"
