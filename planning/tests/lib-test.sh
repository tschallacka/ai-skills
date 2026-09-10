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
#   t_skip <reason>                 a missing precondition (no tool, no
#                                   permission, no capability) ends the test as
#                                   SKIP, not PASS -- call in place of t_end.
#
# Debugging a failure -- full guide in docs/DEBUGGING-TESTS.md, which is the
# single source for the flags and the reasoning. In short:
#
#   (automatic)                     a failing test's root is printed, and kept
#                                   in CI; a local run cleans it up.
#   t_bp <label>                    a named checkpoint; halts when
#                                   AI_SKILLS_TEST_BREAK names it.
#   t_dump [label] [file] [line]    the variables the TEST made, and its files.
#   t_trace_on / t_trace_off        xtrace bounded to a region.
#
# AI_SKILLS_TEST_BREAK_LINE=N (or FILE:N, or a list) stops at a line. Halting
# needs a tty and not-CI; elsewhere it dumps and carries on.
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
# ---- failure evidence, and local debugging ---------------------------------
#
# Two separate jobs, deliberately not one flag:
#
#   EVIDENCE is for CI. A red leg has to be diagnosable from its log alone --
#   there is no re-run, no stepping through, and the runner is destroyed -- so
#   on failure the test root is printed and, in CI, kept. Bounded on purpose:
#   a benchmark root reaches gigabytes, so the dump caps files and bytes rather
#   than emitting a tree nobody will read.
#
#   BREAKPOINTS are for a local run, and are inert anywhere else. They exist so
#   `bash -x` on a whole 400-line test is not the only option: it drowns the
#   thing you are looking for in the 380 lines you are not.
#
# Everything here is opt-in through the environment, so a test file needs no
# changes to benefit and CI behaviour cannot be altered by accident. The flags,
# their defaults and the measurements behind them: docs/DEBUGGING-TESTS.md.
t_evidence_files="${AI_SKILLS_TEST_EVIDENCE_FILES:-40}"
t_evidence_bytes="${AI_SKILLS_TEST_EVIDENCE_BYTES:-16384}"

# The variables that already existed when this library loaded, so a dump can
# show what the TEST made rather than the whole environment. A dump of every
# variable is a dump nobody reads, which is the failure mode being fixed.
t_debug_baseline=" $( (compgen -v 2>/dev/null || true) | LC_ALL=C sort | tr '\n' ' ') "

t_running_in_ci() {
    [ -n "${CI:-}" ] || [ -n "${GITHUB_ACTIONS:-}" ]
}

# Whether a FAILED test's root survives. An explicit flag wins; then a
# deliberate breakpoint stop, because stopping to inspect the tree and then
# deleting it is the one combination nobody wants; then CI keeps and a local
# run cleans.
t_keep_test_root() {
    case "${AI_SKILLS_TEST_KEEP:-}" in
        1|true|yes|always) return 0 ;;
        0|false|no|never)  return 1 ;;
    esac
    [ "${AI_SKILLS_TEST_BREAK_EXIT:-0}" != 0 ] && return 0
    t_running_in_ci
}

# Printable? A core dump or a build artifact in the root would otherwise spray
# the log with control bytes and bury the text files that matter.
#
# NUL bytes, not ASCII-printability: install.sh's own diagnostics use real
# UTF-8 punctuation (an em dash in a soft-requirement warning, for one), which
# `tr -d '[:print:][:space:]'` under LC_ALL=C treats as junk because every
# multi-byte UTF-8 byte has its high bit set -- so a perfectly readable
# install.sh log was misfiled as "(binary, not shown)" on exactly the runs
# that most needed to be read. A core dump or compiled binary reliably carries
# a NUL within its first few bytes; legitimate text, UTF-8 included, never
# does.
t_evidence_is_text() { # <path>
    local total stripped
    total="$(LC_ALL=C head -c 4096 "$1" 2>/dev/null | LC_ALL=C wc -c | tr -d ' ')"
    stripped="$(LC_ALL=C head -c 4096 "$1" 2>/dev/null | LC_ALL=C tr -d '\0' | LC_ALL=C wc -c | tr -d ' ')"
    [ "${total:-0}" = "${stripped:-1}" ]
}

t_evidence_dump() { # <root>
    local root="$1" path size shown=0
    [ -n "$root" ] && [ -d "$root" ] || return 0
    printf '\n--- lib-test evidence: %s ---\n' "$root" >&2
    # A heredoc, not a pipe: `shown` has to survive the loop, and a piped while
    # runs in a subshell where the count is discarded. Filenames with spaces
    # are read intact; one with a newline in it is not, and nothing here makes
    # those.
    while IFS= read -r path; do
        [ -n "$path" ] || continue
        shown=$((shown + 1))
        if [ "$shown" -gt "$t_evidence_files" ]; then
            printf '(more files not shown; raise AI_SKILLS_TEST_EVIDENCE_FILES)\n' >&2
            break
        fi
        size="$(wc -c < "$path" 2>/dev/null | tr -d ' ')"
        printf '\n--- %s (%s bytes) ---\n' "${path#"$root"/}" "${size:-?}" >&2
        if [ "${size:-0}" = 0 ]; then
            printf '(empty)\n' >&2
        elif t_evidence_is_text "$path"; then
            LC_ALL=C head -c "$t_evidence_bytes" "$path" >&2
            [ "${size:-0}" -gt "$t_evidence_bytes" ] && printf '\n(truncated)\n' >&2
        else
            printf '(binary, not shown)\n' >&2
        fi
    done <<T_EVIDENCE_EOF
$(find "$root" -type f 2>/dev/null | LC_ALL=C sort)
T_EVIDENCE_EOF
    printf '\n--- end evidence ---\n' >&2
}

# The variables the test itself created, with long values clipped.
t_dump_vars() {
    local name value
    printf '\n--- lib-test variables ---\n' >&2
    for name in $( (compgen -v 2>/dev/null || true) | LC_ALL=C sort); do
        case "$t_debug_baseline" in *" $name "*) continue ;; esac
        # Locals belonging to the dump machinery itself, not to the test.
        case "$name" in
            name|value|label|answer|rc|t_debug_*|BASH_*|FUNCNAME|PIPESTATUS) continue ;;
        esac
        value="${!name-}"
        if [ "${#value}" -gt 400 ]; then
            value="$(printf '%s' "$value" | LC_ALL=C head -c 400)...(${#value} bytes)"
        fi
        printf '%s=%s\n' "$name" "$value" >&2
    done
}

# Everything known at one point: where we are, what the test made, what is on
# disk. This is the part `bash -x` cannot give you -- it shows the commands,
# never the resulting state.
# The caller passes its own file and line. Both are needed and both were wrong
# before: BASH_LINENO[0] read here is the line of whoever called t_dump, which
# for a t_bp checkpoint is inside this library (it reported "line 179" of
# lib-test.sh), and $0 is always the TEST, so a dump taken inside a library
# claimed to be in the test file.
t_dump() { # [label] [file] [line]
    printf '\n=== lib-test dump: %s (%s line %s) ===\n' \
        "${1:-dump}" "${2:-${0##*/}}" "${3:-${BASH_LINENO[0]:-?}}" >&2
    t_dump_vars
    t_evidence_dump "${T_TMPDIR:-}"
}

# Halt, but only where halting is safe. In CI, or with no terminal on stdin, a
# halt would hang the leg until its timeout and report as something else
# entirely -- so it dumps, says it did not stop, and carries on.
t_break_halt() { # <label>
    if t_running_in_ci || [ ! -t 0 ]; then
        printf 'lib-test: breakpoint %s reached (not halting: CI or no tty)\n' "$1" >&2
        return 0
    fi
    printf 'lib-test: halted at %s -- [enter] continue, s = shell, q = abort > ' "$1" >&2
    local answer=""
    read -r answer || return 0
    case "$answer" in
        s|shell) "${SHELL:-/bin/sh}" ;;
        q|quit|abort) printf 'lib-test: aborted at %s\n' "$1" >&2; exit 70 ;;
    esac
}

# A NAMED checkpoint: `t_bp after-install`, triggered with
# AI_SKILLS_TEST_BREAK="after-install" (or "all"). Preferred over a line
# number, which drifts the moment anyone edits the file above it.
t_bp() { # <label>
    local label="${1:-bp}"
    case " ${AI_SKILLS_TEST_BREAK:-} " in
        *" $label "*|*" all "*) ;;
        *) return 0 ;;
    esac
    t_dump "$label" "${BASH_SOURCE[1]##*/}" "${BASH_LINENO[0]:-?}"
    t_break_halt "$label"
}

# Line breakpoints -- the form an agent debugging a test actually uses. It has
# just read the file, so the line it picks is the line as it stands at that
# moment and cannot have drifted. Accepts a bare LINE, a FILE:LINE, or a
# comma-or-space separated list of either:
#
#     AI_SKILLS_TEST_BREAK_LINE=120,204
#     AI_SKILLS_TEST_BREAK_LINE=lib-test.sh:466
#
# A bare LINE means the running test, which is the common case. The FILE form
# stops inside a library the test calls.
#
# Matching the LINE alone was wrong, and measurably so: `set -T` makes the
# DEBUG trap fire inside every sourced file too, so
# AI_SKILLS_TEST_BREAK_LINE=466 stopped at line 466 of lib-test.sh, inside
# t_end, from a test file five lines long -- and labelled the dump with the
# test's name. A breakpoint that stops somewhere other than where it was set
# and then misreports where it is, is worse than no breakpoint.
#
# One breakpoint firing: dump, then stop or carry on.
# AI_SKILLS_TEST_BREAK_EXIT=1 stops at the first hit with a non-zero exit,
# which also means the root is kept under CI retention -- the shape an agent
# wants when it intends to go and read the tree.
t_break_hit() { # <file:line>
    t_dump "$1" "${1%:*}" "${1##*:}"
    if [ "${AI_SKILLS_TEST_BREAK_EXIT:-0}" != 0 ]; then
        printf 'lib-test: stopped at %s (AI_SKILLS_TEST_BREAK_EXIT)\n' "$1" >&2
        exit 70
    fi
    t_break_halt "$1"
}

# Costly enough to be opt-in only: a DEBUG trap fires before every command.
if [ -n "${AI_SKILLS_TEST_BREAK_LINE:-}" ]; then
    t_break_at=" "
    for t_break_spec in $(printf '%s' "$AI_SKILLS_TEST_BREAK_LINE" | tr ',' ' '); do
        case "$t_break_spec" in
            *:*) t_break_at="$t_break_at$t_break_spec " ;;
            *)   t_break_at="$t_break_at${0##*/}:$t_break_spec " ;;
        esac
    done
    set -T
    # BASH_SUBSHELL guards against firing twice for one line: `set -T`
    # propagates the DEBUG trap into subshells, so a line containing a command
    # substitution hit once for the line and once more inside `$( )`. The
    # trailing `:` keeps the trap's own status zero -- a DEBUG trap returning
    # non-zero can make bash skip the command it was about to run.
    trap 'case "$t_break_at" in *" ${BASH_SOURCE[0]##*/}:${LINENO} "*) if [ "${BASH_SUBSHELL:-0}" = 0 ]; then t_break_hit "${BASH_SOURCE[0]##*/}:${LINENO}"; fi ;; esac; :' DEBUG
fi

# ---- scoped tracing --------------------------------------------------------
#
# `bash -x` on a whole test prints every line of setup, teardown and library
# plumbing; the ten lines actually in question arrive buried. These bound
# xtrace to a region:
#
#     t_trace_on
#     ...the suspect part...
#     t_trace_off
#
# PS4 carries the line number, which bare `set -x` does not, so a trace of a
# helper called from three places says which call it is.
t_trace_on() {
    PS4='+ ${BASH_SOURCE[0]##*/}:${LINENO}: '
    export PS4
    set -x
}

t_trace_off() {
    set +x
}

if [ -z "${T_TMPDIR:-}" ]; then
    # Directly under /tmp, and short. A unix socket path is capped near 104 bytes
    # and chromium (via mmdc) appends about 50 for its profile and singleton
    # socket, so the room a test may use is small. Nesting inside nix develop's
    # TMPDIR *and* run-tests.sh's own scratch reached 75 characters and crossed
    # the limit: test-mermaid-accuracy failed with "Socket path too long" on the
    # bash 3.2 leg only. Measured -- 75 failed, 62 passed -- so this stays far
    # under rather than close to it: /tmp/t.XXXXX is 12 characters.
    #
    # The `t.` prefix is kept so a leaked root is still attributable; the CI leak
    # scan looks for it.
    # An operator-set TMPDIR wins, so a machine whose /tmp is tmpfs can move
    # test DATA to real disk with one export. That is the whole point: a test
    # tree is bytes on a filesystem and has no business being RAM.
    #
    # The socket-path limit above does not justify forcing /tmp on the data
    # root, because only ONE thing in this suite actually needs a short path:
    # chromium's --user-data-dir, where it opens a singleton socket. That now
    # has its own short root below (T_SOCKET_TMPDIR), which is what lets this
    # one honour TMPDIR. Sockets in /tmp, everything else on disk.
    # macOS forces /tmp; only Linux honours TMPDIR here.
    #
    # The reason for honouring it at all is that /tmp is tmpfs on the Linux
    # workstation this is developed on, so a test tree there is RAM. macOS has no
    # such problem, and its own TMPDIR is actively hostile: it points at
    # /var/folders/<...>, and /var is a symlink to /private/var, so a fixture git
    # repo created there has two names and anything comparing paths disagrees
    # with itself. test-atomicity-flow failed on BOTH macOS legs the moment this
    # honoured TMPDIR, while every Linux leg stayed green - a fixture repo whose
    # uncommitted edit git reported under one path and the flow looked for under
    # the other.
    case "$(uname -s)" in
        Darwin) T_TMPDIR="$(mktemp -d /tmp/t.XXXXX)" ;;
        *)      T_TMPDIR="$(mktemp -d "${TMPDIR:-/tmp}/t.XXXXX")" ;;
    esac
    if [ -n "${AI_SKILLS_TEST_RUN_ID:-}" ]; then
        printf '%s\n' "$AI_SKILLS_TEST_RUN_ID" > "$T_TMPDIR/.ai-skills-test-run-id"
    fi

    # Sockets only, and deliberately NOT under TMPDIR. A unix socket path is
    # capped near 104 bytes and chromium (via mmdc) appends about 50 for its
    # profile and singleton socket, so the room a caller has is small: nesting
    # inside nix develop's TMPDIR *and* run-tests.sh's own scratch reached 75
    # characters and crossed the limit, and test-mermaid-accuracy failed with
    # "Socket path too long" on the bash 3.2 leg only. Measured -- 75 failed,
    # 62 passed -- so /tmp/s.XXXXX at 12 characters stays far under rather than
    # close to it. Nothing but a socket or a socket-bearing profile belongs
    # here; it is tmpfs on a developer workstation.
    if [ -d /tmp ] && [ -w /tmp ]; then
        T_SOCKET_TMPDIR="$(mktemp -d /tmp/s.XXXXX)"
    else
        T_SOCKET_TMPDIR="$(mktemp -d "${TMPDIR:-/tmp}/s.XXXXX")"
    fi

    export T_TMPDIR T_SOCKET_TMPDIR
    export TMPDIR="$T_TMPDIR"
    # Cleaned on a LOCAL exit, KEPT on a CI one. Locally the root is noise: the
    # test can be re-run and stepped through, and on this machine these roots
    # are tmpfs, so keeping them is RAM. CI has neither a re-run nor a machine
    # to come back to -- the log and the artifacts are all there will ever be --
    # so nothing there may be deleted before it has been recorded.
    # AI_SKILLS_TEST_KEEP forces it either way. `$$` guards against a subshell
    # running the trap for its parent.
    #
    # The evidence is DUMPED on any failure regardless of retention, because
    # run-tests.sh prints what a test PRINTS and this trap deleted what a test
    # WROTE. docs/DEBUGGING-TESTS.md records what that cost.
    t_tmpdir_owner=$$
    t_tmpdir_cleanup() {
        # First command: $? is the test's exit status and anything else
        # overwrites it. Non-zero is the only failure signal available here,
        # since tests using t_trap_assertions never create a findings file.
        local rc=$?
        [ "$$" = "$t_tmpdir_owner" ] || return 0
        if [ "$rc" -ne 0 ] && [ "${AI_SKILLS_TEST_EVIDENCE:-1}" != 0 ]; then
            t_evidence_dump "${T_TMPDIR:-}"
        fi
        if [ "$rc" -ne 0 ] && t_keep_test_root; then
            printf 'lib-test: kept %s (failed; set AI_SKILLS_TEST_KEEP=0 to clean)\n' \
                "${T_TMPDIR:-}" >&2
            return 0
        fi
        if [ -n "${T_SOCKET_TMPDIR:-}" ]; then
            case "$T_SOCKET_TMPDIR" in /tmp/*|/var/*) rm -rf -- "$T_SOCKET_TMPDIR" ;; esac
        fi
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
    # A setup command dying under set -e used to end a test in silence: the
    # findings file was never printed, so a red leg said nothing (B31). Say
    # which line failed, then let set -e do its job. Guarded probes — `|| rc=$?`,
    # `if ! cmd` — do not fire ERR, so refusal cases stay quiet.
    set -E
    trap 'printf "%s:%s: command failed: %s\n" "${0##*/}" "$LINENO" "$BASH_COMMAND" >&2' ERR
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

# A test whose precondition is absent (no required tool, no permission this
# host will grant, no capability to probe) calls this instead of t_end: "PASS"
# cannot be told apart from "asserted nothing" (B268), since a skip records no
# finding, so this gives a skip its own outcome. Loud on stderr, same as the
# callers already were; run-tests.sh counts a trailing ": SKIP" line here as
# SKIP rather than PASS.
t_skip() {
    printf 'SKIP %s\n' "$*" >&2
    rm -f "${T_FINDINGS:-}"
    printf '%s: SKIP\n' "${0##*/}"
    exit 0
}
