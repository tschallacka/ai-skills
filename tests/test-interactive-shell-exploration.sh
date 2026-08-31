#!/usr/bin/env bash
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
BIN="$ROOT/src/interactive-shell/target/debug"
TARGET="/tmp/interactive-shell-exploration-$$.txt"
RUN_DIR=$(mktemp -d "${TMPDIR:-/tmp}/interactive-shell-exploration.XXXXXX")
SOCKET="$RUN_DIR/socket"
EVENTS="$RUN_DIR/events.jsonl"
PID=

cleanup() {
    if [ -n "$PID" ]; then
        kill "$PID" 2>/dev/null || true
        wait "$PID" 2>/dev/null || true
    fi
    rm -f "$TARGET"
    rm -rf "$RUN_DIR"
}
trap cleanup EXIT INT TERM

wait_screen() {
    needle=$1
    i=0
    while [ "$i" -lt 200 ]; do
        if jq -e --arg needle "$needle" '
            select(.event == "screen" or .event == "snapshot")
            | ((.rows // {}) | to_entries | map(.value) | join("\n"))
            | contains($needle)' "$EVENTS" >/dev/null 2>&1; then
            return 0
        fi
        sleep 0.1
        i=$((i + 1))
    done
    echo "exploration timeout waiting for: $needle" >&2
    tail -5 "$EVENTS" >&2
    return 1
}

send() {
    "$BIN/interactive-shell-input" --socket "$SOCKET" "$@" >/dev/null
    sleep 0.15
}

observe() {
    "$BIN/interactive-shell-input" --socket "$SOCKET" observe | sed -n '1p'
}

if ! command -v mc >/dev/null 2>&1 || ! command -v nano >/dev/null 2>&1 || ! command -v less >/dev/null 2>&1; then
    echo "SKIP: mc, nano, and less are required"
    exit 0
fi

: >"$EVENTS"
"$BIN/interactive-shell" --socket "$SOCKET" --cols 100 --rows 30 --idle-timeout 30 -- \
    nano --ignorercfiles "$TARGET" >"$EVENTS" 2>"$RUN_DIR/nano.err" &
PID=$!
wait_screen 'GNU nano'
send text 'Hello World'
send key CTRL-X
wait_screen 'Save modified buffer'
send raw 59
wait_screen 'Write to File:'
send key ENTER
wait_screen 'Wrote'
send key CTRL-X
wait "$PID" 2>/dev/null || true
PID=

SOCKET="$RUN_DIR/mc.sock"
EVENTS="$RUN_DIR/mc-events.jsonl"
: >"$EVENTS"
EDITOR=nano "$BIN/interactive-shell" --socket "$SOCKET" --cols 120 --rows 35 --idle-timeout 60 -- \
    mc -u /tmp >"$EVENTS" 2>"$RUN_DIR/mc.err" &
PID=$!
wait_screen '/tmp'

found=no
attempt=0
while [ "$attempt" -lt 40 ]; do
    snapshot=$(observe)
    if jq -e --arg file "$TARGET" '
        (.rows // {}) | to_entries[] | select(.value | contains($file))' \
        <<EOF >/dev/null 2>&1
$snapshot
EOF
    then
        target_row=$(jq -r --arg file "$TARGET" '
            (.rows // {}) | to_entries[] | select(.value | contains($file)) | .key' \
            <<EOF
$snapshot
EOF
        )
        cursor_row=$(jq -r '.cursor.row' <<EOF
$snapshot
EOF
        )
        while [ "$cursor_row" -lt "$target_row" ]; do
            send key DOWN
            cursor_row=$((cursor_row + 1))
        done
        while [ "$cursor_row" -gt "$target_row" ]; do
            send key UP
            cursor_row=$((cursor_row - 1))
        done
        found=yes
        break
    fi
    send key PAGEDOWN
    attempt=$((attempt + 1))
done
[ "$found" = yes ]
send key F4
wait_screen 'GNU nano'
send raw 1b5b313b3548
send raw 1b5b313b3246
send text 'hello universe'
send key CTRL-O
wait_screen 'Write to File:'
send key ENTER
if wait_screen 'File exists'; then
    send raw 59
fi
wait_screen 'Wrote'
send key CTRL-X
wait_screen 'GNU nano'
send key CTRL-X
send key F10
wait "$PID" 2>/dev/null || true
PID=
[ "$(cat "$TARGET")" = 'hello universe' ]

SOCKET="$RUN_DIR/less.sock"
EVENTS="$RUN_DIR/less-events.jsonl"
: >"$EVENTS"
"$BIN/interactive-shell" --socket "$SOCKET" --cols 100 --rows 30 --idle-timeout 30 -- \
    less "$TARGET" >"$EVENTS" 2>"$RUN_DIR/less.err" &
PID=$!
wait_screen 'hello universe'
send key q
wait "$PID" 2>/dev/null || true
PID=
echo "interactive-shell observation-driven nano/mc/less exploration passed"
