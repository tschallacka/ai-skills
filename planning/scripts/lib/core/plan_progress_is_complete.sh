#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# plan_progress_is_complete PROGRESS_FILE — succeed only when every goal row
# in a plan progress table reads 100%. An empty roster is not complete.
plan_progress_is_complete() {
    [ -n "${1:-}" ] || { printf 'plan_progress_is_complete: progress file required\n' >&2; return 1; }
    [ -f "$1" ] || { printf 'plan_progress_is_complete: no such file: %s\n' "$1" >&2; return 1; }
    local rows=0
    local line pct
    while IFS= read -r line; do
        case "$line" in '|'*'%'*'|') ;; *) continue ;; esac
        case "$line" in '|'---*'|'|'|required'*|'Required'*) continue ;; esac
        pct=$(plan_table_cell "$line" 3)
        case "$pct" in *%) ;; *) continue ;; esac
        rows=$((rows + 1))
        [ "$pct" = "100%" ] || return 1
    done < "$1"
    [ "$rows" -gt 0 ]
}
