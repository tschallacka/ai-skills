#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: DEV
# Accept --plan-dir <path> wherever the plan directory is positional, because
# plan-context.sh and run-adversary-probe.sh take the flag and a reader who
# learned it there should not have a call refused elsewhere. Prints the argument
# list %q-quoted with the flag's value moved to <position>, so a caller does
# `eval "set -- $(plan_hoist_plan_dir 1 "$@")"` and parses as before.
plan_hoist_plan_dir() {
    local position="$1" hoisted="" arg
    shift
    local rest=()
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --plan-dir)
                [ "$#" -ge 2 ] || plan_die "--plan-dir needs a path"
                hoisted="$2"; shift 2 ;;
            --plan-dir=*)
                hoisted="${1#--plan-dir=}"; shift ;;
            *) rest+=("$1"); shift ;;
        esac
    done
    if [ -n "$hoisted" ]; then
        local out=() i=1
        # PORTABILITY(empty-array-setu)
        for arg in ${rest[@]+"${rest[@]}"}; do
            [ "$i" -ne "$position" ] || out+=("$hoisted")
            out+=("$arg"); i=$((i + 1))
        done
        [ "$i" -gt "$position" ] || out+=("$hoisted")
        rest=(${out[@]+"${out[@]}"})
    fi
    # PORTABILITY(empty-array-setu)
    for arg in ${rest[@]+"${rest[@]}"}; do printf '%q ' "$arg"; done
    printf '\n'
}
