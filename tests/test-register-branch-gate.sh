#!/usr/bin/env bash
# MODE: DEV
# test-register-branch-gate.sh — pre-push-check refuses a register change made
# off the `registers` branch; on it, checks only that nothing but the two
# registers changed, and does so without nix.
#
# BUGS.json and TODO.json are append-mostly arrays, so two branches that each
# file an entry both take the same next free id. Git cannot see that collision:
# the additions land at different array positions, so it merges them textually
# with NO conflict and the result carries two unrelated entries under one id.
# One merge on 2026-09-04 produced eight duplicate ids that way, invisible
# until reg_findings ran, and the resolvers' advice in the textual case is to
# take one side — which drops the other side's entries.
#
# The gate exists so that collision cannot be created. This test exists because
# a gate nobody fault-injects is how several failing-open gates got shipped: it
# drives the real script in a throwaway clone, on the wrong branch and then the
# right one, and asserts the verdict flips. Delete the gate and case 1 fails.
#
# On `registers` the gate is the file scope and nothing else: a push carrying
# any path but BUGS.json and TODO.json fails, and no other gate runs, nor does
# the nix re-entry -- the stale master flake it carries does not build a dev
# shell on Apple Silicon (B333), and this script must still be able to refuse.
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

# The gate can only be exercised where it can actually run, and it re-enters
# `nix develop` unless it is already inside the development shell. On the macOS
# system-bash-3.2 leg there is no nix, so pre-push-check.sh printed
# "nix develop .#default is required for Rust pre-push checks" and exited 69 --
# non-zero, which satisfied the old "was it refused?" assertion for entirely
# the wrong reason, while the two assertions on the refusal TEXT failed. Cases
# 2 and 3 then passed vacuously, because the gate line is absent from a run
# that never started. That is a test reporting FAIL where it should report SKIP,
# and reporting PASS on two checks it did not perform.
#
# So the feature-branch cases (1 and 3) are skipped without nix. The registers
# branch cases (2, 4, 5) need none, and are the ones that must keep running there.
have_nix=1
if [ -z "${AI_SKILLS_PREPUSH_IN_NIX:-}" ] && [ -z "${IN_NIX_SHELL:-}" ] &&
    ! command -v nix >/dev/null 2>&1; then
    have_nix=0
    printf '%s: note: no nix; the feature-branch cases are skipped\n' "${0##*/}"
fi
# `timeout` is GNU coreutils, not stock BSD userland. The CI legs install
# coreutils, so this is for a developer running the suite on a plain Mac; the
# sibling chat tests skip on the same condition.
if ! command -v timeout >/dev/null 2>&1; then
    printf '%s: SKIP (no timeout(1))\n' "${0##*/}"
    exit 0
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/register-branch-gate.XXXXXX")"
trap 'rm -rf "$work"' EXIT

clone="$work/repo"
if ! git clone -q --no-hardlinks --shared "$repo_root" "$clone" 2>/dev/null; then
    printf '%s: SKIP (cannot clone the repo)\n' "${0##*/}"
    exit 0
fi
# The clone carries committed state; the gate under test may be uncommitted.
cp "$repo_root/pre-push-check.sh" "$clone/pre-push-check.sh"
# Committed, and before master is pinned below: on `registers` every changed
# path counts, so an uncommitted copy of the script would itself be refused as
# a stray file.
( cd "$clone" && git add pre-push-check.sh &&
    git -c user.name=t -c user.email=t@example.com commit -q --allow-empty -m 'working-tree gate' )

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
# `for ref in origin/master master`, so origin/master wins and pinning the
# local branch changes nothing.
( cd "$clone" && git update-ref refs/remotes/origin/master HEAD && git branch -f master HEAD )

gate_line='a register is modified outside the registers branch'

# The bash script is what is under test: an empty bin root makes
# plan_exec_compiled_binary_if_present fall through instead of exec'ing whatever
# pre-push-check binary this machine last built (the crate's own integration
# tests cover that one).
mkdir -p "$work/nobin"

run_gate() { # <seconds> -> writes $work/out, echoes the exit code
    local rc=0
    if ( cd "$clone" && AI_SKILLS_BIN_ROOT="$work/nobin" PRE_PUSH_SKIP_FETCH=1 \
        timeout "$1" ./pre-push-check.sh >"$work/out" 2>&1 ); then
        rc=0
    else
        rc=$?
    fi
    printf '%s' "$rc"
}

# A stand-in `nix` that records that it was called and fails, first on PATH and
# with neither marker variable set: the registers branch must never reach it.
mkdir -p "$work/fakebin"
printf '#!/bin/sh\ntouch "%s/nix-was-called"\nexit 97\n' "$work" >"$work/fakebin/nix"
chmod +x "$work/fakebin/nix"

run_registers_gate() { # <seconds> -> writes $work/out, echoes the exit code
    local rc=0
    if ( cd "$clone" && env -u IN_NIX_SHELL -u AI_SKILLS_PREPUSH_IN_NIX \
        PATH="$work/fakebin:$PATH" AI_SKILLS_BIN_ROOT="$work/nobin" PRE_PUSH_SKIP_FETCH=1 \
        timeout "$1" ./pre-push-check.sh >"$work/out" 2>&1 ); then
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
if [ "$have_nix" = 1 ]; then
rc="$(run_gate 120)"
out="$(cat "$work/out")"
# Exit 1 exactly, not merely non-zero. The gate's own `exit 1` is what is being
# asserted; any other non-zero code means the script stopped for some unrelated
# reason and the run proves nothing -- 69 for a missing nix, 127 for a missing
# `timeout`. "Non-zero" accepted all of those as a refusal.
t_assert_eq 'a register change off the registers branch is refused (exit 1)' \
    "$rc" '1'
t_assert_contains 'the refusal names the rule' "$gate_line" "$out"
t_assert_contains 'the refusal names the changed register' 'TODO.json' "$out"
case "$out" in
    *'  ok    '*) t_fail 'another gate reported before the refusal; it is not early' ;;
    *) : ;;
esac
fi

# 2. The same change on the registers branch passes, and nothing else runs: not
#    another gate, and not nix. Both are read from the output and the stand-in,
#    since an exit 0 alone would also come from a script that checked nothing.
( cd "$clone" && git switch -q -c registers )
rc="$(run_registers_gate 60)"
out="$(cat "$work/out")"
t_assert_eq 'a register-only change on the registers branch passes (exit 0)' "$rc" '0'
t_assert_contains 'the pass names the registers it saw' 'only registers changed' "$out"
t_assert_contains 'the pass says no other gate runs' 'no other gate runs' "$out"
case "$out" in
    *"$gate_line"*) t_fail 'the gate fired on the registers branch, where register edits belong' ;;
    *'git diff --check'*|*'PORTABILITY'*|*'cargo'*) t_fail 'an ordinary gate ran on the registers branch' ;;
    *) : ;;
esac
if [ -e "$work/nix-was-called" ]; then
    t_fail 'the registers branch re-entered nix develop'
fi

# 4. Anything but the two registers on the registers branch fails, and names it.
#    The stray file is untracked-then-staged, then edited-and-unstaged, so both
#    halves of the change set are exercised.
( cd "$clone" && printf 'x\n' >stray-file.txt && git add stray-file.txt && printf '\n' >>README.md )
rc="$(run_registers_gate 60)"
out="$(cat "$work/out")"
t_assert_eq 'a non-register change on the registers branch is refused (exit 1)' "$rc" '1'
t_assert_contains 'the refusal names the rule' 'may only change BUGS.json and TODO.json' "$out"
t_assert_contains 'the refusal names a staged stray file' 'stray-file.txt' "$out"
t_assert_contains 'the refusal names an unstaged stray edit' 'README.md' "$out"
if [ -e "$work/nix-was-called" ]; then
    t_fail 'the registers branch re-entered nix develop on a refusal'
fi
( cd "$clone" && git rm -q -f --cached stray-file.txt && rm -f stray-file.txt && git checkout -q -- README.md )

# 5. Nothing changed on the registers branch is a pass, not a refusal.
( cd "$clone" && git checkout -q -- TODO.json )
rc="$(run_registers_gate 60)"
t_assert_eq 'no change at all on the registers branch passes (exit 0)' "$rc" '0'

# 3. A control on the harness itself: with no register change the gate must be
#    silent even on a feature branch. Without this, a gate that fired
#    unconditionally would satisfy case 1 for the wrong reason.
if [ "$have_nix" = 1 ]; then
( cd "$clone" && git switch -q -c feature/no-registers )
run_gate 90 >/dev/null
out="$(cat "$work/out")"
case "$out" in
    *"$gate_line"*) t_fail 'the gate fired with no register change' ;;
    *) : ;;
esac
fi

t_end 'test-register-branch-gate'
