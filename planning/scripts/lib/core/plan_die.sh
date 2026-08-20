#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: DEV
plan_die() {
    printf '%s: %s\n' "${0##*/}" "$1" >&2
    rm -f ${PLAN_DIE_TEMP_FILES[@]+"${PLAN_DIE_TEMP_FILES[@]}"}
    exit "${2:-64}"
}
