#!/usr/bin/env bash
# The repository that owns this plan's snapshots, as create-plan.sh pinned it in
# PLAN_SNAPSHOT_REPO. Returns 1 when there is none, which is the honest answer
# for a plan versioned in a repository the user owns: plan snapshots do not
# belong in that history. The value is read, not re-derived, so a .gitignore
# change after creation cannot move the target silently.
plan_snapshot_repo() {
    local env_file="$1/.env" assignment repo
    [ -f "$env_file" ] || return 1
    assignment="$(grep '^PLAN_SNAPSHOT_REPO=' "$env_file" 2>/dev/null)" || return 1
    # Same character rule plan-env.sh manifest_check enforces on values: a
    # manifest carrying any of these is already invalid, so refuse it here too
    # rather than eval the line.
    case "$assignment" in
        *'$'*|*'`'*|*';'*|*'|'*|*'&'*|*'<'*|*'>'*) return 1 ;;
    esac
    # eval is the inverse of the printf %q that wrote the value, and is what
    # keeps a plans root containing a space working.
    eval "repo=${assignment#PLAN_SNAPSHOT_REPO=}" 2>/dev/null || return 1
    [ -n "$repo" ] || return 1
    printf '%s\n' "$repo"
}
