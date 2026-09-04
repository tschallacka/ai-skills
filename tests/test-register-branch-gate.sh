#!/usr/bin/env bash
# MODE: DEV
# test-register-branch-gate.sh — pre-push-check refuses a register change made
# off the `registers` branch, and stays silent about one made on it.
#
# BUGS.json and TODO.json are append-mostly arrays, so two branches that each
# file an entry both take the same next free id. Git cannot see that collision:
# the additions land at different array positions, so it merges them textually
# with NO conflict and the result carries two unrelated entries under one id.
# One merge on 2026-09-04 produced eight duplicate ids that way, invisible
# until reg_findings ran, and register-resolve.sh's advice in the textual case
# is to take one side — which drops the other side's entries.
#
# The gate exists so that collision cannot be created. This test exists because
# a gate nobody fault-injects is how several failing-open gates got shipped: it
# drives the real script in a throwaway clone, on the wrong branch and then the
# right one, and asserts the verdict flips. Delete the gate and case 1 fails.
#
# Two things this test has to get right, both learned by getting them wrong:
# the clone must carry the WORKING TREE's script, since `git clone` copies only
# committed state and would otherwise test the old one; and a deliberately
# non-zero run must go through `if`, because lib-test's ERR trap records a
# non-zero command as a finding.
#
# Usage:
#   test-register-branch-gate.sh

set -uo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/register-branch-gate.XXXXXX")"
trap 'rm -rf "$work"' EXIT

clone="$work/repo"
if ! git clone -q --no-hardlinks --shared "$repo_root" "$clone" 2>/dev/null; then
    printf '%s: SKIP (cannot clone the repo)\n' "${0##*/}"
    exit 0
fi
# The clone carries committed state; the gate under test may be uncommitted.
cp "$repo_root/pre-push-check.sh" "$clone/pre-push-check.sh"

# The clone's master must BE the branch's base, or the test is not testing what
# it thinks. A clone inherits the source repository's `master` ref, which here
# was many merges behind the checked-out commit, so `master..HEAD` legitimately
# contained register changes and the gate fired on a "clean" branch -- correctly.
# Pinning master to HEAD makes each probe branch genuinely register-free.
#
# PRE_PUSH_SKIP_FETCH=1 goes with it: the script now refreshes origin/master
# before measuring anything, and this clone's origin is a local path whose
# master is the stale one, so a fetch here would undo the pin.
# It must be origin/master, not master: base resolution is
# `for ref in origin/master master`, so origin/master wins and pinning the local
# branch changes nothing. Measured the hard way after two wrong guesses.
( cd "$clone" && git update-ref refs/remotes/origin/master HEAD && git branch -f master HEAD )

gate_line='a register is modified outside the registers branch'

run_gate() { # <seconds> -> writes $work/out, echoes the exit code
    local rc=0
    if ( cd "$clone" && PRE_PUSH_SKIP_FETCH=1 timeout "$1" ./pre-push-check.sh >"$work/out" 2>&1 ); then
        rc=0
    else
        rc=$?
    fi
    printf '%s' "$rc"
}

# 1. A register change on a feature branch is refused, names the register, and
#    refuses BEFORE any other gate reports — the point is not paying for the
#    rust gates first.
( cd "$clone" && git switch -q -c feature/some-work && printf '\n' >>TODO.json )
rc="$(run_gate 120)"
out="$(cat "$work/out")"
t_assert_eq 'a register change off the registers branch is refused' \
    "$( [ "$rc" -ne 0 ] && echo refused || echo allowed )" 'refused'
t_assert_contains 'the refusal names the rule' "$gate_line" "$out"
t_assert_contains 'the refusal names the changed register' 'TODO.json' "$out"
case "$out" in
    *'  ok    '*) t_fail 'another gate reported before the refusal; it is not early' ;;
    *) : ;;
esac

# 2. The same change on the registers branch does not trip this gate. Later gates
#    may still fail in a throwaway clone (an unbuilt tree), so only this gate's
#    verdict is read, and the run is bounded rather than waited out.
( cd "$clone" && git switch -q -c registers )
run_gate 90 >/dev/null
out="$(cat "$work/out")"
case "$out" in
    *"$gate_line"*) t_fail 'the gate fired on the registers branch, where register edits belong' ;;
    *) : ;;
esac

# 3. A control on the harness itself: with no register change the gate must be
#    silent even on a feature branch. Without this, a gate that fired
#    unconditionally would satisfy case 1 for the wrong reason.
( cd "$clone" && git switch -q -c feature/no-registers && git checkout -q -- TODO.json )
run_gate 90 >/dev/null
out="$(cat "$work/out")"
case "$out" in
    *"$gate_line"*) t_fail 'the gate fired with no register change' ;;
    *) : ;;
esac

t_end 'test-register-branch-gate'
