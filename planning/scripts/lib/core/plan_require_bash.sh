#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# Refuse to run under a bash older than <major>. NOT called at load time — this
# library itself is 3.2-clean; a script that genuinely needs bash 4 calls it.
plan_require_bash() {
    local want="$1"
    [ "${BASH_VERSINFO[0]}" -ge "$want" ] || plan_die \
        "needs bash $want or newer (running ${BASH_VERSION:-unknown}); on macOS: brew install bash" 78
}
