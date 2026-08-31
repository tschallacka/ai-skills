#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# plan_crypt_resolve — locate the plan-crypt binary once per process. Sets
# PLAN_CRYPT_RESOLVED to its path and returns 0, or returns 1 when there is
# none. The answer is cached: the lookup is three `command -v`-shaped probes
# and context_hash_file asks for it once per file in a plan.
#
# Opportunistic: a resident compiled helper is used when one is present and
# nothing breaks when it is not. Three places, in order of how deliberately
# each was chosen:
#   1. PLAN_CRYPT_BIN, so a test or a packager can pin one exactly
#   2. anywhere on PATH
#   3. plan_bin_dir — the one directory this installation keeps its compiled
#      helpers in, shared by every skill rather than copied into each. That
#      function owns the whole question of where that is, so this one does not
#      carry its own copy of the platform table or count parent directories.
#
# The cache is keyed on PLAN_CRYPT_BIN as well as on having run, so a test that
# repoints the pin between calls is not served a stale answer.
plan_crypt_resolve() {
    local bin_dir triple candidate exe=''
    # The sentinel is prefixed so it is never empty: an empty marker cannot be
    # told apart from "never run" under set -u, and the very first call would
    # then read an unset PLAN_CRYPT_RESOLVED.
    if [ "${PLAN_CRYPT_RESOLVE_DONE:-}" = "pin:${PLAN_CRYPT_BIN:-}" ]; then
        [ -n "${PLAN_CRYPT_RESOLVED:-}" ] || return 1
        return 0
    fi
    PLAN_CRYPT_RESOLVE_DONE="pin:${PLAN_CRYPT_BIN:-}"
    PLAN_CRYPT_RESOLVED=''
    if [ -n "${PLAN_CRYPT_BIN:-}" ]; then
        [ -x "$PLAN_CRYPT_BIN" ] || return 1
        PLAN_CRYPT_RESOLVED="$PLAN_CRYPT_BIN"
        return 0
    fi
    if command -v plan-crypt >/dev/null 2>&1; then
        PLAN_CRYPT_RESOLVED="$(command -v plan-crypt)"
        return 0
    fi
    triple="$(plan_crypt_target_triple)" || triple=''
    case "$triple" in *-windows-*) exe='.exe' ;; esac
    bin_dir="$(plan_bin_dir)" || return 1
    candidate="$bin_dir/plan-crypt$exe"
    if [ -x "$candidate" ]; then
        PLAN_CRYPT_RESOLVED="$candidate"
        return 0
    fi
    return 1
}
