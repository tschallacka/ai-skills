#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: DEV
plan_decode_escaped_newlines() {
    local value="$1"
    printf '%s' "${value//\\n/$'\n'}"
}
