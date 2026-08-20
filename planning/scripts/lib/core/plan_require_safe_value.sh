#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: DEV
plan_require_safe_value() {
    local label="$1" value="$2"
    [ -n "$value" ] || plan_die "$label must not be empty"
    [[ "$value" != *'|'* ]] || plan_die "$label must not contain a Markdown table separator (|)"
    [[ "$value" != *$'\n'* && "$value" != *$'\r'* ]] || plan_die "$label must be one line"
}
