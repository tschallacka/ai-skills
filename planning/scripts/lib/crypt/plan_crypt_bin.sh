#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# plan_crypt_bin — print the path to the plan-crypt binary, or return 1.
#
# A thin printer over plan_crypt_resolve, for a caller that wants the path
# rather than to use it. The digest and random helpers call the resolver
# directly: this one costs a subshell at every call site, and context_hash_file
# runs once per file in a plan.
plan_crypt_bin() {
    plan_crypt_resolve || return 1
    printf '%s\n' "${PLAN_CRYPT_RESOLVED:-}"
}
