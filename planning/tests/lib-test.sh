#!/usr/bin/env bash
# MODE: DEV
# lib-test — portability shims for the test suites.
#
# Usage: sourced by a test, never executed.
#   t_sed_i <sed-script> <file>     in-place edit, no GNU -i
#   t_sed_insert_before <re> <text> <file>
#   t_stat_mode <file>              octal mode, GNU or BSD stat
#   t_unique_suffix                 a unique token, no `date +%N`
#   t_copy_tree <src> <dst>         contents incl. dotfiles, no `cp -R src/.`
#   t_sha256 <file>                 sha256 hex digest, GNU or BSD or openssl
#
# Assertion support. Two modes, and they are mutually exclusive by design:
#
#   t_trap_assertions               report the failing expression and abort.
#                                   For a test written as bare `[ ... ]` under
#                                   set -e, which otherwise exits 1 in silence.
#   t_begin / t_fail / t_assert_eq / t_assert_contains / t_expect_exit / t_end
#                                   report every finding, then exit once. For a
#                                   new test. Findings go to a FILE, not a
#                                   variable, so a t_fail inside a $( ) is not
#                                   swallowed by the subshell.
#   t_record <message>              record a finding silently, for a test that
#                                   prints its own message and prefix.
#   t_failures                      the recorded count, for a test that prints
#                                   its own epilogue.
#
# The tests run on the same bash 3.2 + BSD floor as the scripts (CI runs the
# suite on macos), so they need the same shims. See PORTABILITY.md; the rule ids
# in the markers below index into it.
# Every test gets its own TMPDIR, assigned here at source time rather than in
# t_begin: one test mktemps before calling it, and the tests using
# t_trap_assertions never call it at all.
#
# Why it matters: planning_tmpdir() is "${TMPDIR:-/tmp}/planning-agent", and the
# fix-key session secrets live under it. Two tests sharing a TMPDIR share that
# directory, and one invalidating a session removes the other's secret. run-tests.sh
# already gives each suite run its own root, so this closes the remaining hole --
# a test invoked directly, which is how anyone debugs one.
#
# The outer leak check in run-tests.sh and in CI still scans the ambient TMPDIR,
# so a test writing outside its own root is still caught. That is now the thing
# worth catching: what a test writes inside its own root is scoped by definition.
if [ -z "${T_TMPDIR:-}" ]; then
    T_TMPDIR="$(mktemp -d "${TMPDIR:-/tmp}/t-$(basename "${BASH_SOURCE[1]:-test}" .sh).XXXXXX")"
    export T_TMPDIR
    export TMPDIR="$T_TMPDIR"
    # Removed on any exit, including a failure: a test that leaves its root
    # behind turns a debugging session into a disk-space problem. `$$` guards
    # against a subshell running the trap for its parent.
    t_tmpdir_owner=$$
    t_tmpdir_cleanup() {
        [ "$$" = "$t_tmpdir_owner" ] || return 0
        [ -n "${T_TMPDIR:-}" ] || return 0
        case "$T_TMPDIR" in /tmp/*|/var/*|"${TMPDIR%/*}"/*) rm -rf -- "$T_TMPDIR" ;; esac
    }
    trap t_tmpdir_cleanup EXIT
fi

# No `set` here: this file is sourced, so changing the caller's shell options
# changes the test's semantics. test-plan-context-paging.sh deliberately runs
# without errexit because it invokes commands that exit non-zero on purpose,
# and inheriting -e from a library aborted it mid-run.

# PORTABILITY(sed-inplace): BSD sed requires a suffix argument for -i, so it
# consumes the script as the suffix and then fails.
t_sed_i() {
    local script="$1" file="$2" temporary
    temporary="$(mktemp "${TMPDIR:-/tmp}/t-sed.XXXXXX")"
    sed "$script" "$file" > "$temporary"
    mv -f "$temporary" "$file"
}

# BSD sed rejects `i text` on one line; awk sidesteps the dialect entirely.
t_sed_insert_before() {
    local pattern="$1" text="$2" file="$3" temporary
    temporary="$(mktemp "${TMPDIR:-/tmp}/t-ins.XXXXXX")"
    awk -v pat="$pattern" -v ins="$text" '
        $0 ~ pat { print ins }
        { print }
    ' "$file" > "$temporary"
    mv -f "$temporary" "$file"
}

# PORTABILITY(stat-format): probe once; GNU takes -c, BSD takes -f.
if stat -c '%a' . >/dev/null 2>&1; then
    t_stat_mode() { stat -c '%a' "$1"; }
else
    t_stat_mode() { stat -f '%Lp' "$1"; }
fi

# PORTABILITY(date-nanoseconds): BSD date has no %N and emits a literal "N".
t_unique_suffix() {
    printf '%s_%s%s' "$$" "${RANDOM}" "${RANDOM}"
}

# PORTABILITY(cp-dot-source): a source path ending in `/.` is unspecified.
# `cp -R src/. dst` creates dst implicitly and tar does not, so mkdir first or
# this is not a faithful replacement.
t_copy_tree() {
    mkdir -p "$2"
    ( cd "$1" && tar cf - . ) | ( cd "$2" && tar xf - )
}

# PORTABILITY(sha256-tool): stock macOS has no sha256sum. Probe once at load, so
# a test does not compare against the empty output of a failed call.
if command -v sha256sum >/dev/null 2>&1; then
    t_sha256() { sha256sum "$1" | awk '{print $1}'; }
elif command -v shasum >/dev/null 2>&1; then
    t_sha256() { shasum -a 256 "$1" | awk '{print $1}'; }
else
    t_sha256() { openssl dgst -sha256 "$1" | awk '{print $NF}'; }
fi

# An ERR trap turns every bare assertion into a reported one, naming the line and
# the expression verbatim. set -E is required or the trap is not inherited into
# functions. This mode aborts at the first failure, which is the price of not
# rewriting the assertions.
t_assertion_failed() {
    printf '%s:%s: assertion failed: %s\n' "${0##*/}" "$1" "$2" >&2
}

t_trap_assertions() {
    set -E
    trap 't_assertion_failed "$LINENO" "$BASH_COMMAND"' ERR
}

# Findings live in a file because a helper called inside a command substitution
# runs in a subshell, where an incremented counter is discarded. That is not
# hypothetical: it made a test's exit-code assertions inert until a mutation
# exposed it.
t_begin() {
    T_FINDINGS="$(mktemp "${TMPDIR:-/tmp}/t-findings.XXXXXX")"
    export T_FINDINGS
}

# Record a finding without printing one. A test that already prints its own
# message -- most do, with a per-test prefix that identifies which test spoke --
# keeps that message and calls this instead of incrementing a local counter,
# which is what a subshell discards.
t_record() {
    printf '%s\n' "${1:-finding}" >> "${T_FINDINGS:?t_begin was not called}"
}

# How many findings have been recorded, for a test that prints its own epilogue.
t_failures() {
    { grep -c . "${T_FINDINGS:?t_begin was not called}" || true; }
}

t_fail() {
    printf 'FAIL: %s\n' "$*" >&2
    t_record "$*"
}

t_assert_eq() { # <label> <actual> <expected>
    [ "$2" = "$3" ] || t_fail "$1: expected '$3', got '$2'"
}

t_assert_contains() { # <label> <needle> <haystack>
    case "$3" in
        *"$2"*) ;;
        *) t_fail "$1: output did not contain '$2'" ;;
    esac
}

t_expect_exit() { # <want-rc> <label> <command...>
    local want="$1" label="$2"
    shift 2
    local rc=0
    "$@" >/dev/null 2>&1 || rc=$?
    [ "$rc" -eq "$want" ] || t_fail "$label: exited $rc, want $want"
}

t_end() {
    local count
    count="$({ grep -c . "${T_FINDINGS:?t_begin was not called}" || true; })"
    rm -f "$T_FINDINGS"
    if [ "${count:-0}" -ne 0 ]; then
        printf '%s: %s failure(s).\n' "${0##*/}" "$count" >&2
        exit 1
    fi
    printf '%s: PASS\n' "${0##*/}"
}
