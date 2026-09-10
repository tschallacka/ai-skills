#!/usr/bin/env bash
# MODE: DEV
# test-run-tests-shard.sh — run-tests.sh's own --list-only/--select-file/--shard
# flags (T116), independent of .github/ci-test-scope.sh's own test: that one
# proves the SELECTOR picks the right tests from a change set; this one proves
# run-tests.sh's OWN flag parsing and shard math are correct for whatever
# list and N they are given, which a CI-only literal (the workflow's
# SHARD_TOTAL) cannot cover on its own.
#
# Every case drives the real script with --list-only, so nothing here actually
# executes a test -- discovery and filtering are proven without paying for a
# full run.
set -uo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
run_tests="$repo_root/run-tests.sh"
work="$(mktemp -d "${TMPDIR:-/tmp}/test-run-tests-shard.XXXXXX")"
trap 'rm -rf "$work"' EXIT
failures=0

note_fail() { printf 'run-tests-shard: %s\n' "$1" >&2; failures=$((failures + 1)); }

full="$("$BASH" "$run_tests" --list-only)"
full_count="$(printf '%s\n' "$full" | awk 'NF' | wc -l | tr -d ' ')"
[ "$full_count" -gt 0 ] || note_fail "--list-only listed nothing at all"

# ---- --list-only is deterministic ------------------------------------------
second="$("$BASH" "$run_tests" --list-only)"
if [ "$full" = "$second" ]; then
    printf '  ok    --list-only is deterministic across two runs\n'
else
    note_fail "--list-only produced a different order/set on a second run"
fi

# ---- --shard partitions the full list, no overlap, no gap ------------------
# N=3 deliberately does not divide most real list sizes evenly, which is the
# case most likely to drop or duplicate an item at the boundary.
n=3
: > "$work/union"
i=0
while [ "$i" -lt "$n" ]; do
    "$BASH" "$run_tests" --list-only --shard "$i/$n" >> "$work/union"
    i=$((i + 1))
done
union_sorted="$(sort "$work/union")"
full_sorted="$(printf '%s\n' "$full" | sort)"
if [ "$union_sorted" = "$full_sorted" ]; then
    printf '  ok    every shard together reconstructs the full list exactly\n'
else
    note_fail "the union of all shards does not equal the full list"
fi

union_unique_count="$(sort -u "$work/union" | awk 'NF' | wc -l | tr -d ' ')"
union_count="$(awk 'NF' "$work/union" | wc -l | tr -d ' ')"
if [ "$union_unique_count" -eq "$union_count" ]; then
    printf '  ok    no item appears in more than one shard\n'
else
    note_fail "an item appeared in more than one shard ($union_count total, $union_unique_count unique)"
fi

# ---- --shard is reproducible: the same I/N always picks the same items -----
first_run="$("$BASH" "$run_tests" --list-only --shard 1/4)"
second_run="$("$BASH" "$run_tests" --list-only --shard 1/4)"
if [ "$first_run" = "$second_run" ]; then
    printf '  ok    the same shard index/total picks the same items every time\n'
else
    note_fail "--shard 1/4 was not reproducible across two runs"
fi

# ---- --select-file restricts to exactly the named items --------------------
select_file="$work/select.txt"
printf '%s\n' "$full" | awk 'NF' | head -3 > "$select_file"
selected="$("$BASH" "$run_tests" --list-only --select-file "$select_file")"
want_sorted="$(sort "$select_file")"
got_sorted="$(printf '%s\n' "$selected" | sort)"
if [ "$want_sorted" = "$got_sorted" ]; then
    printf '  ok    --select-file restricts to exactly the named items\n'
else
    note_fail "--select-file did not restrict to exactly the named items"
fi

# ---- --select-file then --shard shards the REDUCED list, not the full one --
printf '%s\n' "$full" | awk 'NF' | head -6 > "$select_file"
reduced_shard0="$("$BASH" "$run_tests" --list-only --select-file "$select_file" --shard 0/2)"
reduced_shard1="$("$BASH" "$run_tests" --list-only --select-file "$select_file" --shard 1/2)"
reduced_union="$(printf '%s\n%s\n' "$reduced_shard0" "$reduced_shard1" | awk 'NF' | sort)"
reduced_want="$(sort "$select_file")"
if [ "$reduced_union" = "$reduced_want" ]; then
    printf '  ok    --shard divides the --select-file list, not the full one\n'
else
    note_fail "--select-file + --shard did not reconstruct the reduced list"
fi

# ---- refusals ---------------------------------------------------------------
run_expect_usage_error() { # <label> <args...>
    local label="$1"; shift
    local rc=0
    "$BASH" "$run_tests" "$@" >/dev/null 2>&1 || rc=$?
    if [ "$rc" -eq 64 ]; then
        printf '  ok    %s\n' "$label"
    else
        note_fail "$label: exited $rc, expected 64"
    fi
}
run_expect_usage_error "an out-of-range shard index is refused" --shard 5/4
run_expect_usage_error "a non-numeric shard is refused" --shard abc
run_expect_usage_error "a zero shard total is refused" --shard 0/0
run_expect_usage_error "a missing --select-file target is refused" \
    --select-file "$work/does-not-exist.txt"
run_expect_usage_error "an unknown flag is refused" --nonsense

[ "$failures" -eq 0 ] || exit 1
printf '%s\n' 'test-run-tests-shard: PASS'
