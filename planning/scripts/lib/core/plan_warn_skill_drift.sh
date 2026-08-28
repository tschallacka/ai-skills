#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# plan_warn_skill_drift SKILL_DIR — warn on stderr when the running skill is an
# installed copy built from a commit that a reachable canonical checkout does
# not contain. Silent when it cannot tell, which is most of the time.
#
# AI_SKILLS_REPO names the canonical checkout. Without it there is nothing to
# compare against and the check does not guess: a wrong warning about staleness
# is worse than none, because the response to it is to reinstall.
plan_warn_skill_drift() {
    local dir="$1" repo="${AI_SKILLS_REPO:-}" commit
    [ -n "$repo" ] && [ -d "$repo/.git" ] || return 0
    git -C "$dir" rev-parse --git-dir >/dev/null 2>&1 && return 0
    commit="$(sed -n 's/.*commit:\([0-9a-f]*\).*/\1/p' "$dir/.version" 2>/dev/null | head -1)"
    [ -n "$commit" ] || return 0
    if ! git -C "$repo" cat-file -e "$commit^{commit}" 2>/dev/null; then
        printf 'planning: this skill was installed from commit %s, which %s does not contain; reinstall before trusting it\n' \
            "$commit" "$repo" >&2
        return 0
    fi
    git -C "$repo" merge-base --is-ancestor "$commit" HEAD 2>/dev/null || return 0
    if [ "$(git -C "$repo" rev-list --count "$commit"..HEAD 2>/dev/null)" -gt 0 ]; then
        printf 'planning: this skill was installed from %s, %s commit(s) behind %s\n' \
            "$commit" "$(git -C "$repo" rev-list --count "$commit"..HEAD)" "$repo" >&2
    fi
}
