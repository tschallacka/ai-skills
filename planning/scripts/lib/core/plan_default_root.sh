#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
plan_default_root() {
    if [ -n "${PLANS_ROOT:-}" ]; then
        printf '%s\n' "${PLANS_ROOT%/}"
        return 0
    fi
    # The global plans root lives with everything else this toolchain keeps:
    # one tsch-ai-skills home under XDG_CONFIG_HOME (bin/, worktrees/, plans/).
    local base="${XDG_CONFIG_HOME:-}"
    if [ -z "$base" ]; then
        local home_dir="${HOME:-}"
        if [ -z "$home_dir" ] && [ -n "${USERPROFILE:-}" ]; then
            home_dir="$USERPROFILE"
        fi
        if [ -z "$home_dir" ] && [ -n "${HOMEDRIVE:-}${HOMEPATH:-}" ]; then
            home_dir="${HOMEDRIVE:-}${HOMEPATH:-}"
        fi
        [ -n "$home_dir" ] || plan_die "Unable to resolve the user home directory; set PLANS_ROOT"
        base="$home_dir/.config"
    fi
    printf '%s/tsch-ai-skills/plans\n' "${base%/}"
}
