#!/usr/bin/env bash
# MODE: DEV
# test-chat-tail-msgid-cursor.sh - B157/T135: a live `tail`'s cursor advances
# from the server's real msgid tag on each pushed message, not from a
# once-a-second poll.
#
# The two cannot be told apart by end state alone -- both eventually converge
# on the same cursor -- so this asserts on TIMING: several rapid sends, with
# the cursor checked well inside the old poll's 1-second cadence after the
# last one. A poll-only client would not reliably be caught up that fast; a
# tag-driven one updates the instant each line is read. It is a real timing
# assertion, stated as one rather than hidden, the same way ci-scope's
# magnitude heuristic is -- rerun if it ever flakes on a loaded machine.
#
# A missing cargo/rust binaries is a loud SKIP, not a failure.

set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo="$(cd "$root/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$repo/planning/tests" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

SERVER="$repo/target/release/chat-server-rs"
CLIENT="$repo/target/release/chat-client-rs"
if ! command -v cargo >/dev/null 2>&1; then
    prebuilt_server="$(find "$root/bin" -type f -name chat-server-rs 2>/dev/null | head -1)"
    prebuilt_client="$(find "$root/bin" -type f -name chat-client-rs 2>/dev/null | head -1)"
    if [ -n "$prebuilt_server" ] && [ -n "$prebuilt_client" ]; then
        SERVER="$prebuilt_server"
        CLIENT="$prebuilt_client"
    else
        t_skip 'chat tail msgid cursor: no cargo and no prebuilt chat/bin binaries'
    fi
else
    ( cd "$repo/src/chat-server-rs" && cargo build --release >/dev/null 2>&1 ) \
        || t_fail "cargo build chat-server-rs failed"
    ( cd "$repo/src/chat-client-rs" && cargo build --release >/dev/null 2>&1 ) \
        || t_fail "cargo build chat-client-rs failed"
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/chat-cursor.XXXXXX")"
home="$work/home"
mkdir -p "$home"
server_pid=""
tail_pid=""
cleanup() {
    [ -n "$tail_pid" ] && { kill "$tail_pid" 2>/dev/null || :; }
    [ -n "$server_pid" ] && { kill "$server_pid" 2>/dev/null || :; }
    rm -rf "$work"
    return 0
}
trap cleanup EXIT

AI_CHAT_HOME="$home" CHAT_BEACON_PORT=47994 \
    "$SERVER" 0 >"$work/server.out" 2>"$work/server.err" &
server_pid=$!
port=""
for _ in $(seq 1 40); do
    if [ -s "$home/server.port" ]; then
        port="$(cat "$home/server.port")"
        break
    fi
    sleep 0.2
done
case "$port" in
    ''|*[!0-9]*)
        t_fail "server did not report a port"
        t_end
        exit 1
        ;;
esac

chan='#cursorcheck'
chan_log="$home/channels/$chan.log"

# A session (not --no-session) is what gives this test a cursor file to read:
# --no-session means "touch no shared state" (B254/B269) and would leave
# nothing on disk to observe.
tail_dir="$work/c-tail"
mkdir -p "$tail_dir"
AI_CHAT_HOME="$tail_dir" "$CLIENT" tail \
    --server 127.0.0.1:"$port" --nick tailer --chan "$chan" \
    --insecure >"$work/tail.out" 2>"$work/tail.err" &
tail_pid=$!

# Let registration, CAP negotiation and the JOIN complete before anything is
# sent -- a broadcast is not replayed, so a late JOIN would make the rest of
# this test assert nothing. There is no readiness marker to poll here (the
# tail only prints PRIVMSG lines by default), so this is a fixed settle
# window, generous next to the sub-second assertions below.
sleep 1
kill -0 "$tail_pid" 2>/dev/null || t_fail "the tail process exited before it could join: $(cat "$work/tail.err")"

sender_dir="$work/c-send"
mkdir -p "$sender_dir"
send_one() { # <text>
    AI_CHAT_HOME="$sender_dir" timeout 8 "$CLIENT" send \
        --server 127.0.0.1:"$port" --nick sender --chan "$chan" --text "$1" \
        --insecure --no-session >/dev/null 2>&1
}

cursor_of() { # -> the tail's own recorded cursor for $chan, or empty
    AI_CHAT_HOME="$tail_dir" "$CLIENT" session cursor "$chan" 2>/dev/null \
        | awk '{print $2}'
}

# Five rapid sends, back to back, no delay -- well inside the old poll's
# once-a-second cadence.
i=1
while [ "$i" -le 5 ]; do
    send_one "rapid-$i"
    i=$((i + 1))
done

# The cursor must reach the channel's last id (the log's own row count, since
# ids here start at 1 and increment by one per message) well inside 1 second:
# the old poll-only cadence resynchronizes at most once a second, so landing
# here within 400ms is not explainable by polling.
caught_up=false
for _ in $(seq 1 8); do
    recorded="$(cursor_of)"
    actual_max="$(grep -c '^MSG ' "$chan_log" 2>/dev/null || echo 0)"
    if [ -n "$recorded" ] && [ "$recorded" != 0 ] && [ "$recorded" = "$actual_max" ]; then
        caught_up=true
        break
    fi
    sleep 0.05
done
if [ "$caught_up" != true ]; then
    t_fail "the tail's cursor ($(cursor_of)) had not caught up to the channel's last id ($(grep -c '^MSG ' "$chan_log" 2>/dev/null || echo '?')) within 400ms of five rapid sends"
fi

# And the tail is still alive and functioning -- nothing above bought this
# with a panic or an early exit.
kill -0 "$tail_pid" 2>/dev/null || t_fail "the tail process was not still running"
case "$(cat "$work/tail.err" 2>/dev/null)" in
    *panic*) t_fail "the tail logged a panic: $(cat "$work/tail.err")" ;;
esac

[ "$(t_failures)" -eq 0 ] || exit 1
printf '%s\n' 'test-chat-tail-msgid-cursor: PASS'
