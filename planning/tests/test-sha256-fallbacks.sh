#!/usr/bin/env bash
# MODE: DEV
# test-sha256-fallbacks.sh — every branch of the SHA-256 tool chain agrees.
#
# `context_hash_stdin` and `context_hash_file` are plan_sha256_hex, whose chain
# is the plan-crypt binary, then sha256sum, then shasum. It refuses by name when
# none is present. Stock macOS has no sha256sum, so the shasum branch is the one
# that runs there and sha256sum is the only one that ever ran in CI on Linux.
# Nothing exercised the others: a fallback that has never executed is a claim,
# not a capability.
#
# Each shell branch is forced by building a PATH that exposes exactly one of the
# two tools and comparing against a digest taken with the host's own tool; the
# compiled branch is forced by pinning PLAN_CRYPT_BIN. A branch whose tool is
# absent on this host is reported as skipped rather than passed, so the summary
# never overstates what was checked.
#
# openssl was a rung here until plan-crypt replaced it, which is what let the
# openssl row leave planning/requires.tsv. It is deliberately not probed any
# more: a host that has it must not quietly go on passing through it.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
lib="$repo_root/planning/scripts/plan-context-lib.sh"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/sha256-fallbacks.XXXXXX")"
trap 'rm -rf "$work"' EXIT

payload="$work/payload"
printf 'the quick brown fox\nand a second line\n' > "$payload"

# The reference comes from whichever tool this host has, so the test does not
# hard-code a digest that a future payload edit would silently invalidate.
reference="$(t_sha256 "$payload")"
case "$reference" in
    [0-9a-f]*) ;;
    *) t_fail "could not take a reference digest on this host: $reference" ;;
esac

# A PATH holding the shell utilities the library needs, plus one hash tool.
isolated_path() { # <tool>...
    local bin="$work/bin.$1" tool resolved
    rm -rf "$bin"; mkdir -p "$bin"
    for tool in sh awk sed grep cat env printf dirname basename mktemp rm ls "$@"; do
        resolved="$(command -v "$tool" 2>/dev/null)" || continue
        ln -sf "$resolved" "$bin/$tool"
    done
    printf '%s\n' "$bin"
}

# PLAN_CRYPT_BIN is set on every run below, never left to the ambient lookup:
# a pin that names a file which does not exist is a refusal, not a fall-through
# to PATH, so it is how the compiled rung is taken out of the picture. Without
# it a repository with planning/bin/plan-crypt built would silently answer every
# case from the binary and prove nothing about the shell rungs.
hash_with() { # <function> <tool> [argument]
    local function_name="$1" tool="$2" argument="${3:-}" bin
    bin="$(isolated_path "$tool")"
    if [ "$function_name" = context_hash_stdin ]; then
        env -i PATH="$bin" PLAN_CRYPT_BIN="$work/no-such-binary" \
            "$BASH" -c "source '$lib'; context_hash_stdin" < "$payload" 2>&1
    else
        env -i PATH="$bin" PLAN_CRYPT_BIN="$work/no-such-binary" \
            "$BASH" -c "source '$lib'; context_hash_file '$argument'" 2>&1
    fi
}

exercised=0
for tool in sha256sum shasum; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf '%s: skipped, %s is not on this host\n' "${0##*/}" "$tool" >&2
        continue
    fi
    exercised=$((exercised + 1))
    t_assert_eq "context_hash_stdin via $tool" "$(hash_with context_hash_stdin "$tool")" "$reference"
    t_assert_eq "context_hash_file via $tool" "$(hash_with context_hash_file "$tool" "$payload")" "$reference"
done

# ---- the compiled rung, when a build exists --------------------------------
# It answers with no hash tool on PATH at all, which is the whole reason the
# openssl row could leave requires.tsv.
crypt_bin=''
if crypt_bin="$(PLAN_CRYPT_BIN='' "$BASH" -c \
        "source '$repo_root/planning/scripts/plan-crypt-lib.sh'; plan_crypt_bin" 2>/dev/null)" \
        && [ -n "$crypt_bin" ]; then
    exercised=$((exercised + 1))
    bin_bare="$(isolated_path true)"
    t_assert_eq 'context_hash_stdin via plan-crypt, no hash tool on PATH' \
        "$(env -i PATH="$bin_bare" PLAN_CRYPT_BIN="$crypt_bin" \
            "$BASH" -c "source '$lib'; context_hash_stdin" < "$payload" 2>&1)" \
        "$reference"
    t_assert_eq 'context_hash_file via plan-crypt, no hash tool on PATH' \
        "$(env -i PATH="$bin_bare" PLAN_CRYPT_BIN="$crypt_bin" \
            "$BASH" -c "source '$lib'; context_hash_file '$payload'" 2>&1)" \
        "$reference"
else
    printf '%s: skipped, no plan-crypt build here (cargo build --release --manifest-path src/plan-crypt/Cargo.toml)\n' \
        "${0##*/}" >&2
fi

# One agreeing branch proves nothing about the chain: the point is that the
# branches agree with each other.
[ "$exercised" -ge 2 ] \
    || printf '%s: only %s branch(es) available here; build plan-crypt or run on macOS for the rest\n' \
        "${0##*/}" "$exercised" >&2

# ---- and with none of them, it refuses by name -----------------------------
bin_none="$(isolated_path true)"
rc=0
refusal="$(env -i PATH="$bin_none" PLAN_CRYPT_BIN="$work/no-such-binary" \
    "$BASH" -c "source '$lib'; context_hash_stdin" < "$payload" 2>&1)" || rc=$?
[ "$rc" -ne 0 ] || t_fail 'the chain returned success with no hash tool available'
case "$refusal" in
    *'No SHA-256 implementation available'*) ;;
    *) t_fail "the refusal did not name the tools it needs: $refusal" ;;
esac

t_end
