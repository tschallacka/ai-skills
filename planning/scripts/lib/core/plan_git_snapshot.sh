#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: DEV
# Commit the plan before a mutation so every overwrite is recoverable. The
# snapshot lands in the repository PLAN_SNAPSHOT_REPO names, which is usually
# the plans root rather than the plan directory, and the add is scoped to this
# plan so a shared plans-root repo does not sweep up its siblings. No-op without
# git, without a pinned repository, or when that repository is not initialized.
plan_git_snapshot() {
    local plan_dir="$1" repo
    command -v git >/dev/null 2>&1 || return 0
    # Resolve before use: the pathspec below runs with git's cwd set to $repo,
    # so a caller's relative path would be read against the wrong directory and
    # the add would silently match nothing.
    plan_dir="$(cd "$plan_dir" 2>/dev/null && pwd -P)" || return 0
    [ -n "$plan_dir" ] || return 0
    repo="$(plan_snapshot_repo "$plan_dir")" || return 0
    [ -d "$repo/.git" ] || return 0
    git -C "$repo" add -A -- "$plan_dir" >/dev/null 2>&1 || return 0
    git -C "$repo" -c user.name='plan-skill' -c user.email='plan-skill@localhost' \
        commit -q -m "snapshot before ${0##*/}" >/dev/null 2>&1 || true
}
