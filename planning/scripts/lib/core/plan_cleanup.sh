#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
plan_cleanup() {
    local f body
    for f in ${plan_tmp_files[@]+"${plan_tmp_files[@]}"}; do
        [ -z "$f" ] || rm -f -- "$f"
    done
    plan_tmp_files=()
    # Chain the handler that was installed before we were sourced. `trap -p`
    # prints `trap -- 'body' EXIT`; the inner eval strips the single quotes and
    # runs the body exactly once.
    if [ -n "${plan_prior_exit_trap:-}" ]; then
        body="${plan_prior_exit_trap#trap -- }"
        body="${body% EXIT}"
        plan_prior_exit_trap=""
        eval "eval $body"
    fi
}
