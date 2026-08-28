#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# plan_fix_key SECRET MESSAGE — lowercase hex SHA-256 over SECRET||MESSAGE.
#
# The secret is always 64 lowercase hex chars, so the concatenation has exactly
# one split and needs no separator. HMAC was retired (T16): the gate exists to
# stop accidental self-certification by an agent that can read the secret
# anyway, so a plain keyed digest removes the last hard dependency without
# weakening what the gate is for. Keys minted under the retired HMAC scheme do
# not verify and must be re-minted.
#
# One definition, not two. mint-fix-keys.sh and verify-fix-keys.sh each carried
# a copy under a comment saying it "must stay byte-identical to" the other; a
# comment is not a mechanism, and the two copies had to agree for any minted key
# to verify. They now call this.
plan_fix_key() {
    local secret="$1" message="$2"
    printf '%s%s' "$secret" "$message" | plan_sha256_hex
}
