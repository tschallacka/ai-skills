#!/usr/bin/env bash
plan_default_root() {
    if [ -n "${PLANS_ROOT:-}" ]; then
        printf '%s\n' "${PLANS_ROOT%/}"
        return 0
    fi
    local home_dir="${HOME:-}"
    if [ -z "$home_dir" ] && [ -n "${USERPROFILE:-}" ]; then
        home_dir="$USERPROFILE"
    fi
    if [ -z "$home_dir" ] && [ -n "${HOMEDRIVE:-}${HOMEPATH:-}" ]; then
        home_dir="${HOMEDRIVE:-}${HOMEPATH:-}"
    fi
    [ -n "$home_dir" ] || plan_die "Unable to resolve the user home directory; set PLANS_ROOT"
    printf '%s/.plans\n' "${home_dir%/}"
}
