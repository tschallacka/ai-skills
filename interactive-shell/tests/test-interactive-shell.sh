#!/usr/bin/env bash
# MODE: DEV
# test-interactive-shell.sh - drives nano through the PTY wrapper end to end.
set -euo pipefail
# Two levels up: this suite lives at interactive-shell/tests/, so the repo root
# is its grandparent. The crate is a member of the root cargo workspace, so its
# binaries land in the ROOT target directory, never src/interactive-shell/target
# -- and CI sets CARGO_TARGET_DIR, which wins over both.
ROOT=$(cd -- "$(dirname -- "$0")/../.." && pwd)
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/debug"
# Honour TMPDIR: run-tests.sh points it at a per-run scratch it cleans up, and
# a hardcoded /tmp leaks past the run (and /tmp here is a tmpfs, i.e. RAM).
TMP=$(mktemp -d "${TMPDIR:-/tmp}/is-test.XXXXXX")
SOCKET=$TMP/socket
LOG=$TMP/events.jsonl
ERR=$TMP/wrapper.err
CLIENT_LOG=$TMP/client.jsonl
TARGET=$TMP/test.txt
# nano is opened on the BARE NAME from inside $TMP, never on $TARGET's absolute
# path, and the readiness predicate below is why.
#
# nano's title bar holds three fields in the terminal's width -- the version,
# the filename and the modified flag -- and drops the version first when the
# filename will not fit. On Linux $TMPDIR is /tmp, so the path is short and
# "GNU nano" is on screen. On macOS $TMPDIR is a per-user
# /var/folders/xy/<28 chars>/T/ path, run-tests.sh adds its own scratch
# directory and mktemp adds another, and the result is about ninety characters
# in an eighty-column terminal. nano then renders only the left-truncated
# filename:
#
#   "...fhc17x95674wsm_g8s980000gn/T//ai-skills-tests.afGHS0/is-test.osgNlh/test.txt"
#
# so `wait_screen "GNU nano"` timed out after ten seconds on the macOS legs
# while nano was running perfectly -- its shortcut bar was on screen the whole
# time. Widening the predicate would have hidden that the test was asserting
# something about the temp directory's length; shortening the name it opens
# fixes the cause and keeps the predicate strict.
TARGET_NAME=test.txt
# A function rather than a one-line trap string: shellcheck reads the string
# as prose and reported `status` as referenced-but-unassigned (SC2154), which
# the warning-severity gate treats as a failure.
cleanup() {
    local status=$?
    if [ -n "${WRAPPER_PID:-}" ]; then
        kill "$WRAPPER_PID" 2>/dev/null || true
        wait "$WRAPPER_PID" 2>/dev/null || true
    fi
    rm -rf "$TMP"
    exit "$status"
}
trap cleanup EXIT HUP INT TERM

if ! command -v nano >/dev/null 2>&1; then echo "SKIP: nano is unavailable"; exit 0; fi
if ! command -v rjq >/dev/null 2>&1; then echo "SKIP: rjq is unavailable"; exit 0; fi
if [ ! -x "$BIN/interactive-shell" ] || [ ! -x "$BIN/interactive-shell-input" ]; then
    if ! command -v cargo >/dev/null 2>&1; then echo "SKIP: no cargo and no built interactive-shell binaries"; exit 0; fi
    cargo build -p interactive-shell --manifest-path "$ROOT/Cargo.toml" >/dev/null
fi
chmod 700 "$TMP"

# The predicate deadlines are a CEILING, not a sleep: each loop returns the
# moment its condition holds, so a healthy run is no slower for a larger one.
# Ten seconds is ample on a Linux runner and tight on the macOS one, which is
# oversubscribed and pauses for other tenants; only a genuine failure pays the
# full budget, and it pays it once.
#
# rjq, not jq: tests/test-rjq-active-references.sh rejects an active jq call
# site anywhere outside frozen migration evidence, and jq is not in the flake
# dev shell either. rjq reads this JSONL stream and matches jq's -e exit codes
# (0 on output, 4 on none), which is all these predicates rely on.
screen_has() { needle=$1; rjq -e --arg needle "$needle" 'select(.event=="screen" and ([.rows[]?] | any(contains($needle))))' "$LOG" >/dev/null 2>&1; }
# What the terminal last looked like, on stderr, when a predicate times out.
# Without it the failure reads "timed out waiting for <string>" and says nothing
# about what WAS on screen, which is the only thing that identifies the cause --
# and a CI-only failure cannot be re-run interactively to find out.
dump_last_screen() { echo "last screen rows seen:" >&2; rjq 'select(.event=="screen") | .rows[]?' "$LOG" 2>/dev/null | tail -26 | sed 's/^/  | /' >&2; }
wait_screen() { deadline=$(( $(date +%s) + ${IS_TEST_WAIT_SECONDS:-60} )); while [ "$(date +%s)" -lt "$deadline" ]; do screen_has "$1" && return 0; sleep 0.1; done; echo "timed out waiting for screen predicate: $1" >&2; dump_last_screen; return 1; }
wait_screen_any() { deadline=$(( $(date +%s) + ${IS_TEST_WAIT_SECONDS:-60} )); while [ "$(date +%s)" -lt "$deadline" ]; do screen_has "$1" && return 0; screen_has "$2" && return 0; sleep 0.1; done; echo "timed out waiting for screen predicate: $1 or $2" >&2; dump_last_screen; return 1; }
wait_lifecycle() { deadline=$(( $(date +%s) + ${IS_TEST_WAIT_SECONDS:-60} )); while [ "$(date +%s)" -lt "$deadline" ]; do rjq -e 'select(.event=="lifecycle")' "$LOG" >/dev/null 2>&1 && return 0; sleep 0.1; done; return 1; }
send() { "$BIN/interactive-shell-input" --socket "$SOCKET" "$@" | tee -a "$CLIENT_LOG" | rjq -e 'select(.event=="ack")' >/dev/null; }
# `exec` inside the subshell matters: without it $! is the subshell and the
# cleanup trap kills that instead of the wrapper.
start() { : >"$LOG"; : >"$ERR"; ( cd "$TMP" && exec "$BIN/interactive-shell" --socket "$SOCKET" --cols 80 --rows 24 --idle-timeout 20 -- nano --ignorercfiles "$TARGET_NAME" ) >"$LOG" 2>"$ERR" & WRAPPER_PID=$!; }

start; wait_screen "GNU nano"; send text "Hello World"; send key CTRL-X; wait_screen "Save modified buffer"; send raw 59; wait_screen_any "Write to File" "File Name to Write"; send key ENTER; wait_lifecycle; wait "$WRAPPER_PID"; WRAPPER_PID=
[ "$(od -An -tx1 -v "$TARGET" | tr -d ' \n')" = 48656c6c6f20576f726c640a ]

start; wait_screen "GNU nano"; send key META-RIGHT; send key CTRL-E; send key ENTER; send text "hello universe"; send key CTRL-O; wait_screen_any "Write to File" "File Name to Write"; send key ENTER; wait_screen_any "File exists" "Wrote 2 lines"; if screen_has "File exists"; then send raw 59; fi; wait_screen "Wrote"; send key CTRL-X; wait_lifecycle; wait "$WRAPPER_PID"; WRAPPER_PID=
[ "$(od -An -tx1 -v "$TARGET" | tr -d ' \n')" = 48656c6c6f20576f726c640a68656c6c6f20756e6976657273650a ]
echo "interactive shell nano flow passed"
