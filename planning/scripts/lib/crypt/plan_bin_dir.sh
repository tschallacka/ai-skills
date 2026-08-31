#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# plan_bin_dir — print the directory that holds this installation's compiled
# helpers, or return 1 when there is none.
#
# Three places, in the order a running skill should trust them:
#   1. AI_SKILLS_BIN_ROOT, which the global .env manifest exports, so an
#      install states its own answer rather than being guessed at
#   2. the shared install location, one directory for every skill rather than
#      a copy inside each: rjq is a hard requirement of planning, todo and
#      bug-report, and three copies is three chances to ship a stale one
#   3. the development tree's bin/<target triple>, so a checkout that has run
#      ./setup-dev-env.sh exercises the compiled path a target runs
#
# The tree is found by walking UP for a bin/<triple> directory rather than by
# counting parents. A fixed ../../../.. cannot survive being copied between
# directories, and that is not hypothetical: scripts/build-plan-libs.sh
# concatenates scripts/lib/<group>/*.sh into scripts/<group>-lib.sh, two
# segments shallower, so the same literal climbed two directories too far in
# the compiled copy and the bundled rjq was never found (B95).
plan_bin_dir() {
    local triple candidate dir
    if [ -n "${AI_SKILLS_BIN_ROOT:-}" ] && [ -d "$AI_SKILLS_BIN_ROOT" ]; then
        printf '%s\n' "$AI_SKILLS_BIN_ROOT"
        return 0
    fi
    candidate="${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/bin"
    if [ -d "$candidate" ]; then
        printf '%s\n' "$candidate"
        return 0
    fi
    triple="$(plan_crypt_target_triple)" || return 1
    dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd)" || return 1
    while [ -n "$dir" ] && [ "$dir" != / ]; do
        if [ -d "$dir/bin/$triple" ]; then
            printf '%s\n' "$dir/bin/$triple"
            return 0
        fi
        dir="$(dirname "$dir")"
    done
    return 1
}
