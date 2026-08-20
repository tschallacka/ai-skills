#!/usr/bin/env bash
# test-sha256-fallbacks.sh — every branch of the SHA-256 tool chain agrees.
#
# `context_hash_stdin` and `context_hash_file` try sha256sum, then shasum, then
# openssl, and refuse by name when none is present. Stock macOS has no
# sha256sum, so the second branch is the one that runs there and the first is
# the only one that ever ran in CI on Linux. Nothing exercised the others: a
# fallback that has never executed is a claim, not a capability.
#
# Each branch is forced by building a PATH that exposes exactly one of the three
# and comparing against a digest taken with the host's own tool. A branch whose
# tool is absent on this host is reported as skipped rather than passed, so the
# summary never overstates what was checked.

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
    for tool in bash sh awk sed grep cat env printf dirname basename mktemp rm ls "$@"; do
        resolved="$(command -v "$tool" 2>/dev/null)" || continue
        ln -sf "$resolved" "$bin/$tool"
    done
    printf '%s\n' "$bin"
}

hash_with() { # <function> <tool> [argument]
    local function_name="$1" tool="$2" argument="${3:-}" bin
    bin="$(isolated_path "$tool")"
    if [ "$function_name" = context_hash_stdin ]; then
        env -i PATH="$bin" bash -c "source '$lib'; context_hash_stdin" < "$payload" 2>&1
    else
        env -i PATH="$bin" bash -c "source '$lib'; context_hash_file '$argument'" 2>&1
    fi
}

exercised=0
for tool in sha256sum shasum openssl; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf '%s: skipped, %s is not on this host\n' "${0##*/}" "$tool" >&2
        continue
    fi
    exercised=$((exercised + 1))
    t_assert_eq "context_hash_stdin via $tool" "$(hash_with context_hash_stdin "$tool")" "$reference"
    t_assert_eq "context_hash_file via $tool" "$(hash_with context_hash_file "$tool" "$payload")" "$reference"
done

# One agreeing branch proves nothing about the chain: the point is that the
# branches agree with each other.
[ "$exercised" -ge 2 ] \
    || printf '%s: only %s branch(es) available here; run on macOS or a host with openssl for the rest\n' \
        "${0##*/}" "$exercised" >&2

# ---- and with none of the three, it refuses by name ------------------------
bin_none="$(isolated_path true)"
rc=0
refusal="$(env -i PATH="$bin_none" bash -c "source '$lib'; context_hash_stdin" < "$payload" 2>&1)" || rc=$?
[ "$rc" -ne 0 ] || t_fail 'the chain returned success with no hash tool available'
case "$refusal" in
    *'No SHA-256 command available'*) ;;
    *) t_fail "the refusal did not name the tools it needs: $refusal" ;;
esac

t_end
