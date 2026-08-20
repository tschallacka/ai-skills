#!/usr/bin/env bash
plan_decode_escaped_newlines() {
    local value="$1"
    printf '%s' "${value//\\n/$'\n'}"
}
