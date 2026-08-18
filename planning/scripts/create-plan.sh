#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
    echo "Usage: $(basename "$0") <plan-name|plan-directory> <title>" >&2
    echo "  <plan-directory> may be an explicit path (existing behaviour)." >&2
    echo "  <plan-name> (no '/') resolves the plans root via plan-root.sh," >&2
    echo "             prompting on first use in a project." >&2
    exit 64
fi

plan_arg="$1"
title="$2"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"
planning_ensure_tmpdir

case "$plan_arg" in
    */*)
        plan_dir="$plan_arg"
        plans_root="${PLANS_ROOT:-$HOME/.plans}"
        ;;
    *)
        resolved_root="$("$script_dir/plan-root.sh" resolve "${PROJECT_DIR:-$PWD}")"
        plan_dir="$resolved_root/$plan_arg"
        plans_root="$resolved_root"
        ;;
esac

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
    printf '## Environment facts\n\n§ 9.1\n<host or URL to verify on, auth route, and the order in which steps verify against the running application>\n\n'
    printf '## Approach decisions\n\n§ 10.1\n<mechanism choices as prose: where each change lives and why, and alternatives considered and rejected>\n\n'
    printf '## UI classification\n\n- UI affected: no\n- Rationale: <why>\n\n'
    printf '## Adversarial review\n\n- Artifact: `adversarial-review.md`\n- Status: 💤 pending\n'
} > "$temporary_file"
mv "$temporary_file" "$description"
printf '{}\n' > "$plan_dir/commands.json"
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
plan_root=$(cd "$plan_dir" && pwd -P)
"$script_dir/plan-env.sh" write-global "$plans_root" "$(cd "$script_dir/.." && pwd -P)"
"$script_dir/plan-env.sh" write-plan "$plan_root" "$plans_root"
if command -v git >/dev/null 2>&1; then
    # Decide which repo owns the plan's history. A git-excluded plans root
    # (a project's /.plans in .gitignore) or a plans root outside any repo
    # gets its own repo at the root so the whole plans tree is versioned and
    # cross-plan diffs (plan-content.sh diff walking up) work; re-initializes
    # the root repo when its .git is missing. A plan inside an already-versioned
    # tree commits into that tree. An explicit path outside any repo keeps its
    # own per-plan repo (existing behaviour).
    git_repo=""
    if top="$(git -C "$plan_dir" rev-parse --show-toplevel 2>/dev/null)"; then
        if git -C "$top" check-ignore -q "$plan_dir" 2>/dev/null; then
            git_repo="$plans_root"
        else
            git_repo="$top"
        fi
    elif [[ "$plan_arg" != */* ]]; then
        git_repo="$plans_root"
    fi
    if [ -n "$git_repo" ]; then
        mkdir -p "$git_repo"
        git init -q "$git_repo" 2>/dev/null || true
        git -C "$git_repo" add -A -- "$plan_dir" 2>/dev/null || true
        git -C "$git_repo" -c user.name='plan-skill' -c user.email='plan-skill@localhost' \
            commit -q -m "plan: initial structure" 2>/dev/null || true
    else
        git init -q "$plan_dir" 2>/dev/null || true
        git -C "$plan_dir" add -A -- . 2>/dev/null || true
        git -C "$plan_dir" -c user.name='plan-skill' -c user.email='plan-skill@localhost' \
            commit -q -m "plan: initial structure" 2>/dev/null || true
    fi
fi
trap - EXIT
echo "Created $plan_dir"
