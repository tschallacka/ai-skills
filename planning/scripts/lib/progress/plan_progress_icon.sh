#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: DEV
# Status glyph: nothing started, something started, everything done. Written as
# `if` blocks rather than the call sites' `[ … ] && icon=…` chain, which returns
# non-zero under `set -e` when the test fails.
plan_progress_icon() {
    local completed="$1" percent="$2" icon='💤'
    if [ "$completed" -gt 0 ]; then
        icon='⏳'
    fi
    if [ "$percent" -eq 100 ]; then
        icon='✅'
    fi
    printf '%s\n' "$icon"
}
