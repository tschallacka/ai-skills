#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# plan_count_units INVENTORY_FILE — count work-unit rows (| WNN | ...) in a
# work-unit inventory, ignoring DoD coverage rows and separators. Parity with
# the historical awk census is pinned by test-plan-data-lib.sh.
plan_count_units() {
    [ -n "${1:-}" ] || { printf 'plan_count_units: inventory required\n' >&2; return 1; }
    [ -f "$1" ] || { printf 'plan_count_units: no such file: %s\n' "$1" >&2; return 1; }
    local n=0 line id
    while IFS= read -r line; do
        case "$line" in '| W'[0-9]*) ;; *) continue ;; esac
        id=$(plan_table_cell "$line" 2)
        case "$id" in W[0-9][0-9]) n=$((n + 1));; esac
    done < "$1"
    printf '%s\n' "$n"
}
