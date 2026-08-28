#!/usr/bin/env bash
# MODE: DEV
# test-plan-crypt — the fix-key primitives: the compiled rung and the shell
# rungs must produce the same digest, byte for byte, forever.
#
# Usage: test-plan-crypt.sh
#
# This is the one place in the repository where a wrong answer is silent: a
# corrupted digest is still a 64-character hex string, and the gate it feeds
# decides whether an adversarial review can be approved. Two implementations of
# SHA-256 now exist behind plan_sha256_hex — the plan-crypt binary and
# sha256sum/shasum — and keys minted under either must verify under the other.
# So equivalence is asserted, not assumed, over inputs chosen to break a wrong
# one: the empty string, the 55/56/64-byte padding boundary, a multi-block
# message, and the exact shape fix keys are derived over.
#
# The binary rung is exercised only when a build exists (cargo build --release
# --manifest-path src/plan-crypt/Cargo.toml). Absent it the test says so loudly
# and still checks the shell rungs; it never passes in silence.
#
# This file is shipped, so it holds to the shipped-runtime dependency rule in
# CODE-STYLE.md section 1: bash, POSIX coreutils, awk, sed, grep, jq only.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
scripts="$root/planning/scripts"
# shellcheck source=planning/scripts/plan-crypt-lib.sh
source "$scripts/plan-crypt-lib.sh"

work="$T_TMPDIR/plan-crypt"
mkdir -p "$work"

# ---- the corpus ------------------------------------------------------------
# Every entry is a label and a way to produce the input, kept as a here-doc of
# lengths plus a few literals so the file stays readable. `printf '%s'` is the
# producer throughout, matching what fix_key itself does.
# Doubling rather than one concatenation per character: awk's naive loop is
# quadratic and 70000 characters took visible seconds.
repeat() { # <char> <count>
    awk -v c="$1" -v n="$2" 'BEGIN {
        s = c
        while (length(s) < n) s = s s
        printf "%s", substr(s, 1, n)
    }'
}

secret='0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef'

# ---- 1. the binary, if one was built ---------------------------------------
bin=''
if bin="$(plan_crypt_bin 2>/dev/null)"; then
    printf 'plan-crypt: exercising the compiled rung at %s\n' "$bin"
else
    printf 'plan-crypt: SKIP the compiled rung -- no binary found on PATH, at\n'
    printf '  PLAN_CRYPT_BIN, or under planning/bin/. Build one with:\n'
    printf '    cargo build --release --manifest-path src/plan-crypt/Cargo.toml\n'
    printf '    cp src/plan-crypt/target/release/plan-crypt planning/bin/\n'
    bin=''
fi

# shell_digest — the digest the rungs below plan-crypt produce, reached by
# clearing the binary so plan_sha256_hex walks past it. A pin to a nonexistent
# path is refused by plan_crypt_bin, which is exactly the "no binary" state.
shell_digest() {
    PLAN_CRYPT_BIN="$work/absent" plan_sha256_hex
}

binary_digest() {
    PLAN_CRYPT_BIN="$bin" plan_sha256_hex
}

# ---- 2. published vectors --------------------------------------------------
# NIST FIPS 180-2 / CAVS. These pin the algorithm itself: an implementation
# that agreed with a broken twin would still fail here.
check_vector() { # <label> <input> <expected hex>
    local got
    got="$(printf '%s' "$2" | shell_digest)"
    t_assert_eq "shell rung: $1" "$got" "$3"
    if [ -n "$bin" ]; then
        got="$(printf '%s' "$2" | binary_digest)"
        t_assert_eq "binary rung: $1" "$got" "$3"
    fi
}

check_vector 'empty string' '' \
    'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
check_vector 'abc (one block)' 'abc' \
    'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad'
check_vector 'two-block NIST vector' \
    'abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq' \
    '248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1'
check_vector 'multi-block NIST vector' \
    'abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu' \
    'cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1'

# The padding boundary. 55 bytes leaves room for the length field in the same
# block; 56 does not and forces a second; 64 is a whole block plus a whole
# padding block. An off-by-one in the pad calculation shows up here and
# nowhere else.
check_vector '55 bytes (padding fits)' "$(repeat a 55)" \
    '9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318'
check_vector '56 bytes (padding overflows)' "$(repeat a 56)" \
    'b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a'
check_vector '63 bytes' "$(repeat a 63)" \
    '7d3e74a05d7db15bce4ad9ec0658ea98e3f06eeecf16b4c6fff2da457ddc2f34'
check_vector '64 bytes (exact block)' "$(repeat a 64)" \
    'ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb'
check_vector '65 bytes' "$(repeat a 65)" \
    '635361c48bb9eab14198e76ea8ab7f1a41685d6ad62aa9146d301d4f17eb0ae0'

# ---- 3. the two rungs agree ------------------------------------------------
# The vectors above prove each rung right against a published answer. This
# proves them right against each other on inputs no published vector covers:
# the exact strings fix_key derives over. That equivalence is what lets a key
# minted on one machine verify on another.
if [ -n "$bin" ]; then
    for message in \
        '9b7d5e4010a57156|AR-1|W1' \
        'a1b2c3d4e5f60718|AR-42|W7' \
        'ffffffffffffffff|AR-999|W123'; do
        a="$(printf '%s%s' "$secret" "$message" | binary_digest)"
        b="$(printf '%s%s' "$secret" "$message" | shell_digest)"
        t_assert_eq "rungs agree on fix_key input $message" "$a" "$b"
    done
    # Sizes that straddle the streaming buffer as well as the block boundary.
    for len in 1 63 64 65 1000 70000; do
        input="$(repeat x "$len")"
        a="$(printf '%s' "$input" | binary_digest)"
        b="$(printf '%s' "$input" | shell_digest)"
        t_assert_eq "rungs agree on $len bytes" "$a" "$b"
    done
fi

# ---- 4. plan_fix_key is the derivation both scripts use --------------------
# mint-fix-keys.sh and verify-fix-keys.sh each held their own copy under a
# comment saying the two must stay byte-identical. They now call one function;
# assert that neither has grown a private copy back.
for script in mint-fix-keys.sh verify-fix-keys.sh; do
    if grep -q '^fix_key() {' "$scripts/$script"; then
        t_fail "$script defines its own fix_key again; the shared plan_fix_key is the derivation"
    fi
    if ! grep -q 'plan_fix_key' "$scripts/$script"; then
        t_fail "$script does not call plan_fix_key"
    fi
done

t_assert_eq 'plan_fix_key derives secret-then-message' \
    "$(plan_fix_key "$secret" 'sid|AR-1|W1')" \
    "$(printf '%s%s' "$secret" 'sid|AR-1|W1' | shell_digest)"

# Secret first is not decorative: swapping the operands must change the key, or
# a message could stand in for a secret.
if [ "$(plan_fix_key "$secret" 'sid|AR-1|W1')" = "$(plan_fix_key 'sid|AR-1|W1' "$secret")" ]; then
    t_fail 'plan_fix_key is symmetric in its two arguments; the ordering carries no weight'
fi

# ---- 5. openssl is gone ----------------------------------------------------
# The whole point of shipping the binary: the declared requirement went down.
# A reintroduced openssl call would put the row back without anyone noticing.
for script in mint-fix-keys.sh verify-fix-keys.sh; do
    if grep -q 'openssl' "$scripts/$script"; then
        t_fail "$script still invokes openssl; the requires.tsv row was removed on the premise that it does not"
    fi
done
if awk -F'\t' '$1 == "openssl" { found = 1 } END { exit !found }' "$root/planning/requires.tsv"; then
    t_fail 'planning/requires.tsv declares openssl again'
fi

# ---- 6. randomness ---------------------------------------------------------
# The value keys the gate, so the properties that matter are: it is the right
# width, two draws differ, and there is no guessable fallback arm left. The
# retired arm joined a process id, a whole-second clock and two draws of the
# shell's own PRNG with hyphens (B87); a hex draw never contains one.
id_a="$(plan_random_hex 8)"
id_b="$(plan_random_hex 8)"
t_assert_eq 'plan_random_hex 8 is 16 hex chars' "${#id_a}" '16'
t_assert_eq 'plan_random_hex 32 is 64 hex chars' "$(plan_random_hex 32 | tr -d '\n' | wc -c | tr -d ' ')" '64'
if [ "$id_a" = "$id_b" ]; then
    t_fail "two session ids came out identical: $id_a"
fi
case "$id_a" in
    *[!0-9a-f]*) t_fail "plan_random_hex emitted a non-hex character: $id_a" ;;
esac
# 64 draws, all distinct. A source stuck on one value, or seeded from a
# whole-second clock, fails this and passes every single-draw check.
: > "$work/draws"
i=0
while [ "$i" -lt 64 ]; do
    plan_random_hex 8 >> "$work/draws"
    i=$((i + 1))
done
t_assert_eq '64 draws are all distinct' "$(sort -u "$work/draws" | grep -c .)" '64'

# Comment lines are skipped on purpose. The docblocks describe the retired
# fallback rather than quoting it, for exactly this reason -- a ratchet that
# matches file text cannot tell an explanation from a use (B73) -- and this
# assertion must not become the next instance of that.
if awk '/^[[:space:]]*#/ { next } /\$RANDOM/ { found = 1 } END { exit !found }' \
        "$scripts/mint-fix-keys.sh"; then
    t_fail 'mint-fix-keys.sh reads the bash PRNG again; its output must not key the gate (B87)'
fi

# ---- 7. the target triple ---------------------------------------------------
# The five rows rust-development-guidelines.md section 4 declares, and nothing
# else: a machine outside them must fall through rather than probe a directory
# that will never hold a build.
triple="$(plan_crypt_target_triple || printf '')"
case "$(uname -s)" in
    Linux | Darwin)
        case "$triple" in
            x86_64-unknown-linux-musl | aarch64-unknown-linux-musl | \
            x86_64-apple-darwin | aarch64-apple-darwin) ;;
            *) t_fail "plan_crypt_target_triple returned '$triple' on $(uname -s)/$(uname -m)" ;;
        esac
        ;;
esac

# ---- 8. refusal, not improvisation -----------------------------------------
# With no rung at all both scripts must exit 69 rather than mint or report a
# key they cannot derive. The stub PATH holds everything the scripts need
# except a digest tool.
stub="$work/nodigest"
mkdir -p "$stub"
for tool in bash awk sed cat cut head tr mktemp mv rm chmod mkdir dirname grep od wc git printf uname sort comm find date stat; do
    src="$(command -v "$tool" 2>/dev/null)" || continue
    ln -sf "$src" "$stub/$tool"
done
if [ -x "$stub/bash" ] && ! PATH="$stub" command -v sha256sum >/dev/null 2>&1; then
    # mint-fix-keys.sh runs its preflight at the top of the script, before any
    # argument is looked at, so the refusal is reachable without a plan
    # directory. verify-fix-keys.sh runs its own after the ungated early
    # returns — deliberately, so an ungated plan still passes on a machine with
    # no digest tool — and so is not reachable this way.
    rc=0
    refusal="$(PATH="$stub" PLAN_CRYPT_BIN="$work/absent" "$stub/bash" \
        "$scripts/mint-fix-keys.sh" --plan-dir "$work/no-such-plan" 2>&1)" || rc=$?
    t_assert_eq 'mint with no digest rung exits 69' "$rc" '69'
    t_assert_contains 'mint names the three rungs' \
        'plan-crypt binary, sha256sum, or shasum' "$refusal"
else
    printf 'plan-crypt: SKIP the starved-PATH refusal check (no usable stub PATH here)\n'
fi

t_end
