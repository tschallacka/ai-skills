#!/usr/bin/env bash
# Flag-coverage test — CRITICAL contract:
#   1. Every script must accept --help and exit 0 with non-empty help output.
#   2. That help output must mention every flag the script's parser accepts
#      (a flag with no help documentation is a CRITICAL failure).
#   3. Every accepted flag must be referenced by at least one test
#      (a flag with no test is a CRITICAL failure).
# Library files (sourced, not standalone executables) are exempt from having
# their own --help but must still have their accepted flags tested.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scripts="$root/scripts"
tests="$root/tests"

# Sourced libraries are not run standalone and need no --help. Membership is the
# `*-lib.sh` suffix, not a hand-kept list: a list silently demands a --help from
# every library left off it.
HELP_FLAGS='-h --help'

extract_flags() {
    grep -hoE '^\s*((--[a-z][a-z0-9-]*|-[a-zA-Z])(\|(--[a-z][a-z0-9-]*|-[a-zA-Z]))*)\)' "$1" \
        | sed -E 's/^\s*//; s/\)$//' \
        | tr '|' '\n' \
        | grep -E '^--?[a-zA-Z]' \
        | sort -u || true
}

critical=0
checked=0
for script in "$scripts"/*.sh; do
    name="$(basename "$script")"
    is_lib=0
    case "$name" in *-lib.sh) is_lib=1 ;; esac
    flags="$(extract_flags "$script")"
    [ -n "$flags" ] || [ "$is_lib" -eq 1 ] || continue

    # CRITICAL 1: --help must be present and functional (exit 0, non-empty).
    if [ "$is_lib" -eq 0 ]; then
        set +e
        helpout="$(bash "$script" --help 2>&1)"
        rc=$?
        set -e
        if [ "$rc" -ne 0 ] || [ -z "$helpout" ]; then
            echo "CRITICAL: $name --help is not functional (rc=$rc, output=${#helpout} bytes)" >&2
            critical=$((critical + 1))
        else
            # CRITICAL 2: help must document every accepted flag.
            for flag in $flags; do
                case " $HELP_FLAGS " in *" $flag "*) continue ;; esac
                checked=$((checked + 1))
                # PORTABILITY(pipefail-grep-q): -w has no case equivalent, so grep
                # stays — with -c, which drains the pipe.
                if ! printf '%s' "$helpout" | grep -cw -- "$flag" >/dev/null; then
                    echo "CRITICAL: $name --help does not document accepted flag '$flag'" >&2
                    critical=$((critical + 1))
                fi
            done
        fi
    fi

    # CRITICAL 3: every accepted flag must be referenced by a test.
    for flag in $flags; do
        case " $HELP_FLAGS " in *" $flag "*) continue ;; esac
        if ! grep -rqw -- "$flag" "$tests" 2>/dev/null; then
            echo "CRITICAL: $name accepts '$flag' but no test references it" >&2
            critical=$((critical + 1))
        fi
    done
done

echo "flag-coverage: $checked flags documented-checked; $critical critical failures"
[ "$critical" -eq 0 ] && echo "test-flag-coverage: PASS" || exit 1