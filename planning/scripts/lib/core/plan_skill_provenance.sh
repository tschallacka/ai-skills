#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# plan_skill_provenance SKILL_DIR — one line naming which build of the planning
# skill is running, on stdout. Empty and non-zero when it cannot tell.
#
# The failure this answers was silent: an installed copy 290 lines adrift of
# canonical looked and behaved like a working skill, and the drift only surfaced
# when a reader concluded the reader itself was missing a feature and hand-
# patched around it (T52). Nothing anywhere said which build was speaking.
#
# Two shapes. A checkout reports its commit, which is exact. An installed copy
# reports what the installer recorded in .version at install time, which is the
# commit it was BUILT from and says nothing about what canonical holds now — so
# the wording distinguishes them rather than printing one number for both.
plan_skill_provenance() {
    local dir="$1" head marker package commit
    if head="$(git -C "$dir" rev-parse --short HEAD 2>/dev/null)"; then
        printf 'planning skill: checkout at %s\n' "$head"
        return 0
    fi
    marker="$dir/.version"
    [ -f "$marker" ] || return 1
    package="$(sed -n 's/^package_version=//p' "$marker" | head -1)"
    commit="$(sed -n 's/.*commit:\([0-9a-f]*\).*/\1/p' "$marker" | head -1)"
    [ -n "$package$commit" ] || return 1
    printf 'planning skill: installed build %s, from commit %s\n' \
        "${package:-unknown}" "${commit:-unknown}"
}
