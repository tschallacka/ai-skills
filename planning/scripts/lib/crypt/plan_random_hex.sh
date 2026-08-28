#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# plan_random_hex BYTES — BYTES bytes from the OS CSPRNG, as lowercase hex.
#
# Two rungs and no third. plan-crypt reads /dev/urandom or BCryptGenRandom
# directly; without it, this reads /dev/urandom itself. There is deliberately
# no "if neither, improvise" arm: the values this produces are the session id
# and the session secret that key the adversarial-review fix-key gate, and the
# arm it replaces was guessable from a process id, a clock truncated to a whole
# second, and a 15-bit shell PRNG a reader can reseed (B77).
# Failing loudly is the only honest option; a caller that cannot get entropy
# must refuse to mint, not mint something weak.
#
# Returns 69 (EX_UNAVAILABLE) when no source is readable.
plan_random_hex() {
    local bytes="$1" out
    if plan_crypt_resolve; then
        "$PLAN_CRYPT_RESOLVED" random-hex "$bytes"
        return $?
    fi
    [ -r /dev/urandom ] || return 69
    # od -An -vtx1 is POSIX and prints one space-separated hex pair per byte on
    # GNU and BSD alike; -v keeps repeated bytes rather than collapsing them to
    # a "*" line, which would silently shorten the output.
    out="$(head -c "$bytes" /dev/urandom | od -An -vtx1 | tr -d ' \n')"
    # A short read must not pass as a weaker key: two hex chars per byte.
    [ "${#out}" -eq "$((bytes * 2))" ] || return 69
    printf '%s\n' "$out"
}
