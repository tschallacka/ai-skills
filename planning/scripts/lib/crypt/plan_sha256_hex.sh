#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# plan_sha256_hex — read stdin, print its lowercase hex SHA-256.
#
# The chain is plan-crypt, then sha256sum, then shasum. openssl was the third
# rung and is gone: plan-crypt covers the machine that has neither coreutils'
# nor perl's digest tool, so the openssl row left planning/requires.tsv with
# this change rather than being replaced by another one.
#
# The compiled rung and the shell rungs are two implementations of one
# algorithm and could in principle disagree. That is answered by measurement,
# not by picking one: planning/tests/test-plan-crypt.sh asserts they produce
# identical hex over a corpus that includes the padding boundary, so a
# divergence fails the suite instead of silently invalidating minted keys.
#
# Returns 69 (EX_UNAVAILABLE) when no rung exists, so a caller can refuse
# rather than mint a key that would never verify.
plan_sha256_hex() {
    if plan_crypt_resolve; then
        "$PLAN_CRYPT_RESOLVED" sha256
    elif command -v sha256sum >/dev/null 2>&1; then
        sha256sum | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 | awk '{print $1}'
    else
        return 69
    fi
}
