#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# Ensure the planning scratch directory exists for this boot. Failure is
# ignored: the helpers still work when a nonstandard TMPDIR is unwritable.
planning_ensure_tmpdir() {
    local d
    d="$(planning_tmpdir)"
    mkdir -p "$d" 2>/dev/null && chmod 700 "$d" 2>/dev/null || true
}
