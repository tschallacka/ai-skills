#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# plan_crypt_resolve — locate the plan-crypt binary once per process. Sets
# PLAN_CRYPT_RESOLVED to its path and returns 0, or returns 1 when there is
# none. The answer is cached: the lookup is three `command -v`-shaped probes
# and context_hash_file asks for it once per file in a plan.
#
# Opportunistic, the same shape as chat-server.sh's rust_bin(): a resident
# compiled helper is used when one is present and nothing breaks when it is
# not. Three places, in order of how deliberately each was chosen:
#   1. PLAN_CRYPT_BIN, so a test or a packager can pin one exactly
#   2. anywhere on PATH
#   3. the skill's own bin/ — <skill>/bin/<target triple>/plan-crypt is the
#      layout rust-development-guidelines.md section 6 declares for a shipped
#      artifact; <skill>/bin/plan-crypt is what a local `cargo build` drop
#      produces, and is looked at second so a shipped per-target binary wins.
#
# The cache is keyed on PLAN_CRYPT_BIN as well as on having run, so a test that
# repoints the pin between calls is not served a stale answer.
plan_crypt_resolve() {
    local skill_dir triple candidate exe=''
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
    # BASH_SOURCE names the compiled library, planning/scripts/plan-crypt-lib.sh,
    # so one level up is the skill directory.
    skill_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." 2>/dev/null && pwd)" || return 1
    for candidate in \
        ${triple:+"$skill_dir/bin/$triple/plan-crypt$exe"} \
        "$skill_dir/bin/plan-crypt$exe"; do
        if [ -x "$candidate" ]; then
            PLAN_CRYPT_RESOLVED="$candidate"
            return 0
        fi
    done
    return 1
}
