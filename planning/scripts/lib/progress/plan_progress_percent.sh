#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# ── Progress rendering ───────────────────────────────────────────────────────
# Half-up rounding (+ total / 2) and the 20-column default width are contract:
# every caller must render byte-identical output.
plan_progress_percent() {
    local completed="$1" total="$2"
    if [ "$total" -gt 0 ]; then
        printf '%s\n' "$(( (completed * 100 + total / 2) / total ))"
    else
        printf '0\n'
    fi
}
