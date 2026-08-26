#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# plan_cycle_count DOC_FILE — count '## Cycle N' review-round headings.
# CRLF is normalised before counting; an empty or missing document yields 0.
plan_cycle_count() {
    [ -n "${1:-}" ] || { printf 'plan_cycle_count: document required\n' >&2; return 1; }
    [ -f "$1" ] || { printf '%s\n' 0; return 0; }
    tr -d '\r' < "$1" | grep -c '^## Cycle [0-9]' || true
}
