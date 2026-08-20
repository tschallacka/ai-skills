#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
plan_decode_escaped_newlines() {
    local value="$1"
    printf '%s' "${value//\\n/$'\n'}"
}
