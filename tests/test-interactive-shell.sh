#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd -- "$(dirname -- "$0")/.." && pwd)
CRATE=$ROOT/src/interactive-shell
BIN=$CRATE/target/debug
TMP=$(mktemp -d /tmp/is-test.XXXXXX)
SOCKET=$TMP/socket
LOG=$TMP/events.jsonl
ERR=$TMP/wrapper.err
CLIENT_LOG=$TMP/client.jsonl
TARGET=$TMP/test.txt
trap 'status=$?; if [ -n "${WRAPPER_PID:-}" ]; then kill "$WRAPPER_PID" 2>/dev/null || true; wait "$WRAPPER_PID" 2>/dev/null || true; fi; rm -rf "$TMP"; exit "$status"' EXIT HUP INT TERM

if ! command -v nano >/dev/null 2>&1; then echo "SKIP: nano is unavailable"; exit 0; fi
if [ ! -x "$BIN/interactive-shell" ] || [ ! -x "$BIN/interactive-shell-input" ]; then cargo build --locked --manifest-path "$CRATE/Cargo.toml" >/dev/null; fi
chmod 700 "$TMP"

screen_has() { needle=$1; jq -e --arg needle "$needle" 'select(.event=="screen" and ([.rows[]?] | any(contains($needle))))' "$LOG" >/dev/null 2>&1; }
wait_screen() { deadline=$(( $(date +%s) + 10 )); while [ "$(date +%s)" -lt "$deadline" ]; do screen_has "$1" && return 0; sleep 0.1; done; echo "timed out waiting for screen predicate: $1" >&2; return 1; }
wait_lifecycle() { deadline=$(( $(date +%s) + 10 )); while [ "$(date +%s)" -lt "$deadline" ]; do jq -e 'select(.event=="lifecycle")' "$LOG" >/dev/null 2>&1 && return 0; sleep 0.1; done; return 1; }
send() { "$BIN/interactive-shell-input" --socket "$SOCKET" "$@" | tee -a "$CLIENT_LOG" | jq -e 'select(.event=="ack")' >/dev/null; }
start() { : >"$LOG"; : >"$ERR"; "$BIN/interactive-shell" --socket "$SOCKET" --cols 80 --rows 24 --idle-timeout 20 -- nano --ignorercfiles "$TARGET" >"$LOG" 2>"$ERR" & WRAPPER_PID=$!; }

start; wait_screen "GNU nano"; send text "Hello World"; send key CTRL-X; wait_screen "Save modified buffer"; send raw 59; wait_screen "Write to File:"; send key ENTER; wait_lifecycle; wait "$WRAPPER_PID"; WRAPPER_PID=
[ "$(od -An -tx1 -v "$TARGET" | tr -d ' \n')" = 48656c6c6f20576f726c640a ]

start; wait_screen "GNU nano"; send key META-RIGHT; send key CTRL-E; send key ENTER; send text "hello universe"; send key CTRL-O; wait_screen "Write to File:"; send key ENTER; wait_screen "File exists" || wait_screen "Wrote 2 lines"; if screen_has "File exists"; then send raw 59; fi; wait_screen "Wrote"; send key CTRL-X; wait_lifecycle; wait "$WRAPPER_PID"; WRAPPER_PID=
[ "$(od -An -tx1 -v "$TARGET" | tr -d ' \n')" = 48656c6c6f20576f726c640a68656c6c6f20756e6976657273650a ]
echo "interactive shell nano flow passed"
