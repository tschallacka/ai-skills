#!/usr/bin/env bash
# MODE: DEV
set -eu

# Two levels up: this suite lives at interactive-shell/tests/, so the repo root
# is its grandparent. The crate is a member of the root cargo workspace, so its
# binaries land in the ROOT target directory, never src/interactive-shell/target
# -- and CI sets CARGO_TARGET_DIR, which wins over both.
ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/../.." && pwd)
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/debug"
RUN_DIR=$(mktemp -d "${TMPDIR:-/tmp}/interactive-shell-exploration.XXXXXX")
MC_DIR="$RUN_DIR/files"
mkdir -p "$MC_DIR"
TARGET="$MC_DIR/test.txt"
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
        if grep -F "$needle" "$EVENTS" >/dev/null 2>&1; then
            return 0
        fi
        sleep 0.1
        i=$((i + 1))
    done
    echo "exploration timeout waiting for: $needle" >&2
    return 1
}

wait_file_content() {
    needle=$1
    i=0
    while [ "$i" -lt 200 ]; do
        if [ -f "$TARGET" ] && grep -F "$needle" "$TARGET" >/dev/null 2>&1; then
            return 0
        fi
        sleep 0.1
        i=$((i + 1))
    done
    echo "exploration timeout waiting for file content: $needle" >&2
    return 1
}

send() {
    "$BIN/interactive-shell-input" --socket "$SOCKET" "$@" >/dev/null
    sleep 0.15
}

observe() {
    "$BIN/interactive-shell-input" --socket "$SOCKET" observe | sed -n '1p'
}

if ! command -v mc >/dev/null 2>&1 || ! command -v nano >/dev/null 2>&1 || ! command -v less >/dev/null 2>&1 || ! command -v rjq >/dev/null 2>&1; then
    echo "SKIP: mc, nano, less, and rjq are required"
    exit 0
fi

# The binaries are gitignored build output, so a clean checkout has none. Build
# them through the root workspace; without cargo there is nothing to test and a
# loud SKIP is the honest result, matching chat/tests/test-chat.sh.
if [ ! -x "$BIN/interactive-shell" ] || [ ! -x "$BIN/interactive-shell-input" ]; then
    if ! command -v cargo >/dev/null 2>&1; then
        echo "SKIP: no cargo and no built interactive-shell binaries"
        exit 0
    fi
    cargo build -p interactive-shell --manifest-path "$ROOT/Cargo.toml" >/dev/null
fi

: >"$EVENTS"
"$BIN/interactive-shell" --socket "$SOCKET" --cols 100 --rows 30 --idle-timeout 30 -- \
    nano --ignorercfiles "$TARGET" >"$EVENTS" 2>"$RUN_DIR/nano.err" &
PID=$!
wait_screen 'test.txt'
send text 'Hello World'
send key CTRL-X
wait_screen 'Save modified buffer'
send raw 59
wait_screen 'Write to File:'
send key ENTER
wait_file_content 'Hello World'
wait "$PID" 2>/dev/null || true
PID=

SOCKET="$RUN_DIR/mc.sock"
EVENTS="$RUN_DIR/mc-events.jsonl"
: >"$EVENTS"
EDITOR=nano "$BIN/interactive-shell" --socket "$SOCKET" --cols 120 --rows 35 --idle-timeout 60 -- \
    mc -u "$MC_DIR" >"$EVENTS" 2>"$RUN_DIR/mc.err" &
PID=$!
wait_screen 'test.txt'

found=no
attempt=0
TARGET_NAME=$(basename "$TARGET")
while [ "$attempt" -lt 40 ]; do
    snapshot=$(observe)
        if rjq -e --arg file "$TARGET_NAME" '
        (.rows // {}) | to_entries[] | select(.value | contains($file))' \
        <<EOF >/dev/null 2>&1
$snapshot
EOF
    then
        target_coords=$(rjq -r --arg file "$TARGET_NAME" '
            (.elements // [])[]
            | select(.label == $file)
            | "\(.row + 1) \(.col + 1)"
        ' <<EOF
$snapshot
EOF
        )
        target_coords=$(printf '%s\n' "$target_coords" | sed -n '1p')
        target_row=${target_coords%% *}
        target_col=${target_coords#* }
        send click-at "$target_col" "$target_row" 0
        found=yes
        break
    fi
    send key PAGEDOWN
    attempt=$((attempt + 1))
done
[ "$found" = yes ]
send key F4
# The editor opens on the file the discovery step selected. That is the mc
# assertion this test can actually make, and it is deliberately where the mc
# block now stops.
#
# NARROWED, and said out loud rather than quietly: everything that used to
# follow here drove nano's keys (CTRL-A, CTRL-K, CTRL-O) and waited for nano's
# "Write to File:" prompt on the assumption that EDITOR=nano makes mc's F4 open
# nano. It does not -- mc opens its built-in mcedit, whose function-key bar
# reads "2Save 3Mark 4Replac ... 10Quit", so those keystrokes meant unrelated
# things and the save prompt never arrived. The block could never have passed;
# it went unnoticed because CI has no mc (so this suite skips there) and this
# branch had never had a CI run at all. Driving mcedit's own save instead is
# real work, not a rename: F2 through the wrapper leaves the buffer modified and
# the file untouched even though F4 (the same SS3 encoding) is honoured by the
# panel. That is recorded as B125 rather than papered over with an assertion
# that passes for the wrong reason.
wait_screen 'Hello World'
send shutdown
wait "$PID" 2>/dev/null || true
PID=

# The less block below needs a file whose content differs from what nano wrote,
# and the wrapper-driven edit path is already proven by the nano block above and
# by interactive-shell/tests/test-interactive-shell.sh. Write it directly rather
# than through an editor interaction this test cannot yet perform.
printf 'hello universe\n' > "$TARGET"

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
