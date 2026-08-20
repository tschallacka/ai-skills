#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: PROD
# validate-plan-common-lib.sh — the finding vocabulary every other
# validate-plan pass calls into: FAIL/WARN reporting, cell trimming, heading
# assertions, and single-field extraction.
#
# Sourced by validate-plan.sh; never executed.
#
# Contracts other libraries depend on:
#   1. `fail` increments the caller's `errors` counter; `warn` NEVER does.
#      The "--complete promotes a WARN to a FAIL" pattern in the placeholder,
#      command-literal and adversarial-review passes is built on that split, so
#      making `warn` count would turn every mid-draft advisory into a gate.
#   2. `get_single_field` returns by global: it sets `field_value` (empty when
#      the field is absent or duplicated) rather than printing, so a caller can
#      tell "absent" from "present but empty" without a subshell.

set -euo pipefail

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    errors=$((errors + 1))
}

warn() {
    printf 'WARN: %s\n' "$*" >&2
}

trim() {
    local value="$1"
    value="${value#"${value%%[![:space:]]*}"}"
    value="${value%"${value##*[![:space:]]}"}"
    value="${value#\`}"; value="${value%\`}"
    printf '%s' "$value"
}

require_heading() {
    local file="$1" heading="$2"
    grep -Fqx "$heading" "$file" || fail "Missing '$heading' in $file"
}

field_value=''

get_single_field() {
    local file="$1" label="$2"
    local count value
    count="$(grep -Ec "^[[:space:]]*-[[:space:]]*${label}:[[:space:]]*.+[[:space:]]*$" "$file" || true)"
    if [ "$count" -ne 1 ]; then
        fail "$file must declare exactly one '$label:' field (found $count)"
        field_value=''
        return
    fi
    value="$(sed -nE "s/^[[:space:]]*-[[:space:]]*${label}:[[:space:]]*(.*[^[:space:]])[[:space:]]*$/\1/p" "$file")"
    field_value="$(trim "$value")"
}
