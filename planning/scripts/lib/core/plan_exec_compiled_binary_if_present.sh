#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# plan_exec_compiled_binary_if_present <binary-name> <caller-script-dir> "$@" —
# exec the compiled <binary-name> when this installation has one (a dev tree
# after ./setup-dev-env.sh, or a packaged release), with the caller's own
# argv, exactly as it received it; otherwise fall through to the caller's own
# bash implementation, completely unchanged.
#
# One owner for a block two dispatcher scripts each carried as a hand-written
# copy: 45+ more scripts are due to wire onto this pattern, and copy-pasting
# it into each is the same "one owner per concern" violation paid for
# elsewhere. Mirrors plan_crypt_resolve.sh's own resolve-then-fall-through
# shape: nothing breaks when no binary is present.
#
# PLANNING_SKILL_ROOT is exported unconditionally before the exec: walk UP
# from caller_script_dir (itself included) for the nearest ancestor
# containing a planning/scripts subdirectory -- matching skill_root()'s own
# convention on the binary side, so the two agree by construction rather
# than a depth constant staying in sync with wherever a caller lives. NOT a
# fixed two-levels-up computation: that worked for every caller through goal
# 13 (planning/scripts, ci-failures/scripts, both two levels below root) but
# broke goal 14's pre-push-check.sh, wired AT the repo root itself (B344 only
# changed which fixed depth was hardcoded, never made it depth-independent;
# found by adversarial review, confirmed by tracing the arithmetic directly).
# Prints nothing and returns 1 if no such ancestor exists short of /, which
# the caller treats as "could not resolve." A binary that never reads the
# variable (confirmed for update-plan-content) simply ignores it.
#
# Sourcing plan-crypt-lib.sh is harmless even when the caller sources it
# again later (only (re)defines functions, nothing readonly); every name it
# introduced, including PLAN_CRYPT_LIB_LOADED, is unset on fall-through.
#
# NOT harmless, and not fixable from inside this function: plan-crypt-lib.sh
# sets `set -euo pipefail` at its own top, and a `source` shares this shell's
# option state rather than a subshell's -- so a caller that deliberately runs
# without -e (run-tests.sh's own `set -uo pipefail`, so one failing
# build-tool call does not abort the whole suite before its summary prints)
# has that silently revoked on the fall-through path, regardless of whether
# the exec above ever runs (found in goal 15/T145: this exact leak turned an
# unguarded generate-portability.sh failure into a whole-script abort). Fix
# belongs at the CALL SITE, not here, since -e is already on by the time this
# function is even entered: force your own options back immediately after
# calling this function (unreached on the exec path) -- e.g. run-tests.sh's
# own `set +e; set -uo pipefail`. Skip it if `-euo pipefail` is already what
# you want (ci-failures.sh, update-plan-content.sh, validate-plan.sh).
#
# Resolved relative to THIS function's own definition file (BASH_SOURCE[0]
# in a function is always where it was defined, not called from), not
# caller_script_dir (B344/AR-39): that crashed unconditionally for any
# caller outside planning/scripts/, where plan-crypt-lib.sh does not exist.
# This function ships bundled into plan-core-lib.sh, so BASH_SOURCE[0]
# resolves to that bundle's own path, a fixed sibling of plan-crypt-lib.sh
# regardless of which directory the caller lives in.
pecbip_find_skill_root() {
    local dir="$1"
    while :; do
        [ -d "$dir/planning/scripts" ] && { printf '%s\n' "$dir"; return 0; }
        [ "$dir" = / ] && return 1
        dir="$(dirname "$dir")"
    done
}

# pecbip_pick <bin-dir, or empty> <name> <caller-script-dir> -- print the
# executable to run, or return 1.
#
# It is `<name>` in the bin directory plan_bin_dir chose, then, last, beside the
# wrapper itself (B365). An installed skill carries its compiled commands in its
# own scripts/ directory, next to the .sh wrappers that front them, while
# plan_bin_dir answers with the first directory that EXISTS, not the first that
# holds this binary -- so the moment a shared bin exists every installed wrapper
# would otherwise fall through to the exit-69 branch with its binary sitting
# next to it. Beside the wrapper comes only after the override, the shared bin
# and the development tree, so none of those can be shadowed by a stray copy.
#
# A Windows build is `<name>.exe`. Git for Windows' bash usually resolves
# `<name>` to it on its own, but asking for the suffixed name outright does not
# depend on that, and costs nothing anywhere else.
pecbip_pick() {
    local dir candidate side
    side="$(cd "$3" && pwd)"
    for dir in "$1" "$side"; do
        [ -n "$dir" ] || continue
        for candidate in "$2" "$2.exe"; do
            if [ -f "$dir/$candidate" ] && [ -x "$dir/$candidate" ]; then
                printf '%s\n' "$dir/$candidate"
                return 0
            fi
        done
    done
    return 1
}

plan_exec_compiled_binary_if_present() {
    local binary_name="$1" caller_script_dir="$2" pecbip_bin_dir pecbip_lib_dir pecbip_skill_root pecbip_target
    shift 2
    pecbip_lib_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    # shellcheck source=planning/scripts/plan-crypt-lib.sh
    source "$pecbip_lib_dir/plan-crypt-lib.sh"
    pecbip_bin_dir="$(plan_bin_dir)" || pecbip_bin_dir=""
    if pecbip_target="$(pecbip_pick "$pecbip_bin_dir" "$binary_name" "$caller_script_dir")"; then
        pecbip_skill_root="$(pecbip_find_skill_root "$(cd "$caller_script_dir" && pwd)")" || pecbip_skill_root=""
        PLANNING_SKILL_ROOT="$pecbip_skill_root" \
            exec "$pecbip_target" "$@"
    fi
    unset -f plan_bin_dir plan_crypt_bin plan_crypt_resolve plan_crypt_target_triple \
        plan_fix_key plan_random_hex plan_sha256_chain plan_sha256_hex pecbip_find_skill_root pecbip_pick
    unset PLAN_CRYPT_LIB_LOADED
}
