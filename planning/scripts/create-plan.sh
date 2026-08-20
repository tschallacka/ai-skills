#!/usr/bin/env bash
# MODE: PROD
# create-plan.sh — create a plan directory with its plan-description.md,
# work-unit-inventory.md, commands.json, env files, and initial git commit.
#
# Where the plan lands depends on the argument: a path (containing "/") is used
# verbatim, a bare name resolves the plans root via plan-root.sh, prompting on
# first use in a project. Which repository owns the plan's history is decided
# further down, next to the rules it follows.
#
# Usage:
#   create-plan.sh <plan-name|plan-directory> <title>
#   create-plan.sh --help

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} <plan-name|plan-directory> <title>
       ${0##*/} --help

  <plan-directory>  an explicit path (existing behaviour).
  <plan-name>       no '/': resolves the plans root via plan-root.sh,
                    prompting on first use in a project.
USAGE
    exit "$rc"
}

case "${1:-}" in -h|--help) usage 0 ;; esac
[ "$#" -eq 2 ] || usage

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
if [ -e "$plan_dir" ]; then
    printf '%s: %s\n' "${0##*/}" "Plan directory already exists: $plan_dir" >&2
    exit 73
fi
plan_require_safe_value title "$title"

# ---- refuse while any plan under this root holds a duplicate step number ----
# Two steps numbered alike in one goal have no defined order, and there is no
# renumbering helper, so the repair is a rename the calling agent has to make.
# Blocking creation is deliberate and it is loud: a broken plan left behind while
# new work starts elsewhere is how the state survives, and the agent that can fix
# it is the one standing here.
duplicate_report=""
for existing_plan in "$plans_root"/*/; do
    [ -d "$existing_plan" ] || continue
    while IFS= read -r collision; do
        [ -n "$collision" ] || continue
        duplicate_report="$duplicate_report$(printf '\n  %s: goal %s' "$(basename "${existing_plan%/}")" "$collision")"
    done <<COLLISIONS
$(plan_duplicate_step_numbers "${existing_plan%/}")
COLLISIONS
done
if [ -n "$duplicate_report" ]; then
    {
        printf '\n'
        printf '%s\n' '================================================================'
        printf '%s\n' 'REFUSING TO CREATE A PLAN: a plan under this root has two steps'
        printf '%s\n' 'sharing one number, so their execution order is undefined.'
        printf '%s\n' '================================================================'
        printf 'Plans root: %s\n' "$plans_root"
        printf 'Collisions (goal, number, then the colliding files):%s\n' "$duplicate_report"
        printf '\n'
        printf '%s\n' 'Rename one of each pair to a free number, then create this plan.'
        printf '%s\n' 'A step rename touches five surfaces: the step file, its testing'
        printf '%s\n' 'companion, the inventory row File cell, the goal owned-unit blurb,'
        printf '%s\n' 'and any progress tracker naming the step. Sweep all five.'
        printf '%s\n' 'plan-content.sh find <plan> <step-name> --in all lists them.'
        printf '\n'
    } >&2
    exit 73
fi

mkdir -p "$plan_dir"
description="$plan_dir/plan-description.md"
# The trap unwinds a partial creation (it also removes the new plan directory),
# so it is released on success rather than left to run.
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
plans_root_path=$(cd "$plans_root" && pwd -P)

# Which repository owns this plan's history, and which owns its pre-mutation
# snapshots. Both are decided here, before the manifests are written, because
# PLAN_SNAPSHOT_REPO pins the answer for the life of the plan.
git_available=false
git_repo=""
snapshot_repo=""
if command -v git >/dev/null 2>&1; then
    git_available=true
    # A git-excluded plans root, or one outside any repo, gets its own repo at
    # the root so cross-plan diffs work; a plan inside an already-versioned
    # tree commits into that tree instead.
    if top="$(git -C "$plan_dir" rev-parse --show-toplevel 2>/dev/null)"; then
        if git -C "$top" check-ignore -q "$plan_dir" 2>/dev/null; then
            git_repo="$plans_root"
        else
            git_repo="$top"
        fi
    elif [[ "$plan_arg" != */* ]]; then
        git_repo="$plans_root"
    fi
    # A repository at the plans root or at the plan itself is ours to commit
    # into on every mutation. The enclosing work tree of a tracked plan is the
    # user's, so it takes the initial commit and nothing after it.
    if [ -n "$git_repo" ]; then
        case "$(cd "$git_repo" 2>/dev/null && pwd -P)" in
            "$plans_root_path") snapshot_repo="$plans_root_path" ;;
        esac
    else
        snapshot_repo="$plan_root"
    fi
fi

"$script_dir/plan-env.sh" write-global "$plans_root" "$(cd "$script_dir/.." && pwd -P)"
"$script_dir/plan-env.sh" write-plan "$plan_root" "$plans_root" "$snapshot_repo"
if [ "$git_available" = true ]; then
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
printf 'Created %s\n' "$plan_dir"
