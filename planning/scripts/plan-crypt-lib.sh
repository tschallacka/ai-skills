#!/usr/bin/env bash
# MODE: PROD
# GENERATED FILE — do not edit. Compiled from scripts/lib/crypt/*.sh by:
#   planning/scripts/build-plan-libs.sh
# Edit the function file in that directory, then re-run the build.
# Target: prod
#
# SHA-256 digests, fix-key derivation and OS entropy

set -euo pipefail

[ -z "${PLAN_CRYPT_LIB_LOADED:-}" ] || return 0
PLAN_CRYPT_LIB_LOADED=1

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

# plan_crypt_resolve — locate the plan-crypt binary once per process. Sets
# PLAN_CRYPT_RESOLVED to its path and returns 0, or returns 1 when there is
# none. The answer is cached: the lookup is three `command -v`-shaped probes
# and context_hash_file asks for it once per file in a plan.
#
# Opportunistic, the same shape as chat-server.sh's rust_bin(): a resident
# compiled helper is used when one is present and nothing breaks when it is
# not. Three places, in order of how deliberately each was chosen:
#   1. PLAN_CRYPT_BIN, so a test or a packager can pin one exactly
#   2. anywhere on PATH
#   3. the skill's own bin/ — <skill>/bin/<target triple>/plan-crypt is the
#      layout rust-development-guidelines.md section 6 declares for a shipped
#      artifact; <skill>/bin/plan-crypt is what a local `cargo build` drop
#      produces, and is looked at second so a shipped per-target binary wins.
#
# The cache is keyed on PLAN_CRYPT_BIN as well as on having run, so a test that
# repoints the pin between calls is not served a stale answer.
plan_crypt_resolve() {
    local skill_dir triple candidate exe=''
    # The sentinel is prefixed so it is never empty: an empty marker cannot be
    # told apart from "never run" under set -u, and the very first call would
    # then read an unset PLAN_CRYPT_RESOLVED.
    if [ "${PLAN_CRYPT_RESOLVE_DONE:-}" = "pin:${PLAN_CRYPT_BIN:-}" ]; then
        [ -n "${PLAN_CRYPT_RESOLVED:-}" ] || return 1
        return 0
    fi
    PLAN_CRYPT_RESOLVE_DONE="pin:${PLAN_CRYPT_BIN:-}"
    PLAN_CRYPT_RESOLVED=''
    if [ -n "${PLAN_CRYPT_BIN:-}" ]; then
        [ -x "$PLAN_CRYPT_BIN" ] || return 1
        PLAN_CRYPT_RESOLVED="$PLAN_CRYPT_BIN"
        return 0
    fi
    if command -v plan-crypt >/dev/null 2>&1; then
        PLAN_CRYPT_RESOLVED="$(command -v plan-crypt)"
        return 0
    fi
    triple="$(plan_crypt_target_triple)" || triple=''
    case "$triple" in *-windows-*) exe='.exe' ;; esac
    # BASH_SOURCE names the compiled library, planning/scripts/plan-crypt-lib.sh,
    # so one level up is the skill directory.
    skill_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." 2>/dev/null && pwd)" || return 1
    for candidate in \
        ${triple:+"$skill_dir/bin/$triple/plan-crypt$exe"} \
        "$skill_dir/bin/plan-crypt$exe"; do
        if [ -x "$candidate" ]; then
            PLAN_CRYPT_RESOLVED="$candidate"
            return 0
        fi
    done
    return 1
}

# plan_crypt_target_triple — the Rust target triple for this machine, or
# nothing plus exit 1 on a platform the house target list does not cover.
#
# The five rows are exactly the ones rust-development-guidelines.md section 4
# declares, so a machine outside them has no shipped binary to find and must
# fall through to the shell rungs rather than probing a directory that will
# never exist. Linux maps to musl because that is the only Linux row we build:
# a musl binary is static, so it runs on a glibc host too.
plan_crypt_target_triple() {
    local os arch
    os="$(uname -s 2>/dev/null || printf 'unknown')"
    arch="$(uname -m 2>/dev/null || printf 'unknown')"
    case "$os" in
        Linux)
            case "$arch" in
                x86_64 | amd64) printf 'x86_64-unknown-linux-musl\n' ;;
                aarch64 | arm64) printf 'aarch64-unknown-linux-musl\n' ;;
                *) return 1 ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64) printf 'x86_64-apple-darwin\n' ;;
                arm64 | aarch64) printf 'aarch64-apple-darwin\n' ;;
                *) return 1 ;;
            esac
            ;;
        # Git Bash, MSYS2 and Cygwin all run the msvc binary; there is no
        # separate Cygwin row, per the same section's tier-3 rule.
        MINGW* | MSYS* | CYGWIN*)
            case "$arch" in
                x86_64 | amd64) printf 'x86_64-pc-windows-msvc\n' ;;
                *) return 1 ;;
            esac
            ;;
        *) return 1 ;;
    esac
    return 0
}

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

# plan_random_hex BYTES — BYTES bytes from the OS CSPRNG, as lowercase hex.
#
# Two rungs and no third. plan-crypt reads /dev/urandom or BCryptGenRandom
# directly; without it, this reads /dev/urandom itself. There is deliberately
# no "if neither, improvise" arm: the values this produces are the session id
# and the session secret that key the adversarial-review fix-key gate, and the
# arm it replaces was guessable from a process id, a clock truncated to a whole
# second, and a 15-bit shell PRNG a reader can reseed (B87).
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

# plan_sha256_chain — the rungs plan_sha256_hex walks, named for a human.
#
# A refusal has to say what to install, so the message has to name the tools.
# Spelling them out at each call site put four more files in front of the
# portability catalogue's probe rule, which matches file text and cannot tell a
# refusal message from a call. Naming them once, here, beside the probe that is
# already exempt for the same reason, keeps every caller's message accurate and
# leaves one file to exempt instead of five.
plan_sha256_chain() {
    printf 'the plan-crypt binary, sha256sum, or shasum\n'
}

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
