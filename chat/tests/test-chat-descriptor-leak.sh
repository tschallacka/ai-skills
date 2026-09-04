#!/usr/bin/env bash
# MODE: DEV
# test-chat-descriptor-leak.sh - the server must not leak a descriptor per
# connection, and must survive running out of them.
#
# B127. `chat-client-rs send` ends with QUIT, and the server's QUIT handler
# returned from its connection loop instead of falling through to the teardown
# at the bottom of it. So the socket was never closed: one leaked descriptor per
# message on the bus. When the process reached its descriptor limit, accept()
# began failing, every later connection was dropped without being serviced, and
# the message it carried was never appended - the sender saw a timeout and the
# message was simply gone. Nothing was written anywhere to say why, and only a
# restart cleared it.
#
# Measured against the server before the fix, with its limit set to 40:
# sends 1-32 succeeded, 33-50 all failed with "no echo from server", and the
# channel log held 32 of 50 messages. At `ulimit -n 6` the process aborted
# outright with `Os { code: 24, TooManyOpenFiles }` from an unwrap in the accept
# loop - and this crate is built with `panic = "abort"`, so that killed every
# other live connection too.
#
# The descriptor limit is the whole method here: the leak is one descriptor per
# connection either way, and lowering the ceiling turns "fails after a thousand
# messages, days from now" into "fails after thirty, in ten seconds".
#
# To watch every assertion fail, point it at a server built before the fix:
#   CHAT_SERVER_BIN=/path/to/old/chat-server-rs chat/tests/test-chat-descriptor-leak.sh

set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo="$(cd "$root/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$repo/planning/tests" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

SERVER="$repo/target/release/chat-server-rs"
CLIENT="$repo/target/release/chat-client-rs"

if ! command -v timeout >/dev/null 2>&1; then
    printf 'SKIP chat descriptor leak: no timeout(1) - the send assertions need a bounded wait\n' >&2
    t_end
    exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
    prebuilt_server="$(find "$root/bin" -type f -name chat-server-rs 2>/dev/null | head -1)"
    prebuilt_client="$(find "$root/bin" -type f -name chat-client-rs 2>/dev/null | head -1)"
    if [ -n "$prebuilt_server" ] && [ -n "$prebuilt_client" ]; then
        SERVER="$prebuilt_server"
        CLIENT="$prebuilt_client"
    else
        printf 'SKIP chat descriptor leak: no cargo and no prebuilt chat/bin binaries\n' >&2
        t_end
        exit 0
    fi
else
    ( cd "$repo/src/chat-server-rs" && cargo build --release >/dev/null 2>&1 ) \
        || t_fail "cargo build chat-server-rs failed"
    ( cd "$repo/src/chat-client-rs" && cargo build --release >/dev/null 2>&1 ) \
        || t_fail "cargo build chat-client-rs failed"
fi
SERVER="${CHAT_SERVER_BIN:-$SERVER}"

work="$(mktemp -d "${TMPDIR:-/tmp}/chat-fd.XXXXXX")"
server_pid=""
cleanup() {
    # `|| :` and an explicit `return 0`: a teardown kill must not decide the
    # verdict when the process has already gone.
    [ -n "$server_pid" ] && { kill "$server_pid" 2>/dev/null || :; }
    rm -rf "$work"
    return 0
}
trap cleanup EXIT

# Start a server with a descriptor limit, and report the port it bound.
# A limit the host refuses (or that the server cannot start under) is a SKIP
# for that assertion, not a failure: the point is the leak, not the exact
# number a platform allows.
start_server() { # <tag> <fd-limit> -> echoes the port, empty on no-start
    # One `local` per name that depends on an earlier one: in a single `local`
    # the earlier assignment has not taken effect yet, so `home` came out as
    # "$work/" and the server started in the wrong directory. That made this
    # whole test pass in a quarter of a second having asserted nothing.
    local tag="$1"
    local limit="$2"
    local home="$work/$tag"
    local port=""
    mkdir -p "$home"
    ( ulimit -n "$limit" 2>/dev/null || exit 70
      exec env AI_CHAT_HOME="$home" "$SERVER" 0 \
          >"$work/$tag.out" 2>"$work/$tag.err" ) &
    printf '%s' "$!" >"$work/$tag.pid"
    local i=0
    while [ "$i" -lt 40 ]; do
        if [ -s "$home/server.port" ]; then
            port="$(cat "$home/server.port")"
            break
        fi
        sleep 0.2
        i=$((i + 1))
    done
    printf '%s' "$port"
}

send_one() { # <port> <nick> <chan> <text> -> 0 when the send completed
    local port="$1" nick="$2" chan="$3" text="$4"
    local dir="$work/c-$nick"
    mkdir -p "$dir"
    AI_CHAT_HOME="$dir" timeout 20 "$CLIENT" send \
        --server 127.0.0.1:"$port" --nick "$nick" --chan "$chan" --text "$text" \
        --insecure --no-session >"$work/send-$nick.out" 2>&1
}

# 1. The symptom, and the assertion that matters: with a low ceiling, every
#    message must still be accepted and appended. Before the fix this stopped
#    dead at the ceiling and silently lost everything after it.
if ! ( ulimit -n 40 ) 2>/dev/null; then
    printf 'SKIP chat descriptor leak: this host will not accept ulimit -n 40\n' >&2
    t_end
    exit 0
fi
port="$(start_server low 40)"
server_pid="$(cat "$work/low.pid" 2>/dev/null)"
if [ -z "$port" ]; then
    t_fail "the server did not start under a 40-descriptor limit: $(cat "$work/low.err" 2>/dev/null)"
else
    i=1
    sent_ok=0
    while [ "$i" -le 50 ]; do
        send_one "$port" "fd$i" '#fd' "message-$i" && sent_ok=$((sent_ok + 1))
        i=$((i + 1))
    done
    logged=0
    [ -f "$work/low/channels/#fd.log" ] && logged="$(wc -l <"$work/low/channels/#fd.log" | tr -d ' ')"
    [ "$sent_ok" = 50 ] \
        || t_fail "only $sent_ok of 50 sends completed under a 40-descriptor limit; first failure: $(cat "$work/send-fd$((sent_ok + 1)).out" 2>/dev/null)"
    [ "$logged" = 50 ] \
        || t_fail "only $logged of 50 messages were appended under a 40-descriptor limit - the rest were accepted-and-lost"

    # 2. The leak itself, named rather than inferred from the symptom. Linux
    #    only: it reads the process's own descriptor table.
    if [ -d "/proc/$server_pid/fd" ]; then
        after="$(find "/proc/$server_pid/fd" -mindepth 1 -maxdepth 1 2>/dev/null | wc -l | tr -d ' ')"
        # Stdio, the listener, the announce socket and a little slack. The
        # failing shape is linear growth - 50 connections meant +50 - so a
        # generous ceiling still catches it.
        [ "$after" -le 12 ] \
            || t_fail "the server holds $after descriptors after 50 connections (expected a handful): one is leaking per connection"
    else
        printf 'SKIP chat descriptor leak: no /proc, so the descriptor count itself was not asserted\n' >&2
    fi
fi
kill "$server_pid" 2>/dev/null || :
server_pid=""

# 3. Running out of descriptors must be survivable and must be reported. The
#    server used to abort here, and `panic = "abort"` takes every other live
#    connection with it. A limit this low may be refused by the host, so a
#    no-start is a SKIP.
port="$(start_server tiny 6)"
tiny_pid="$(cat "$work/tiny.pid" 2>/dev/null)"
server_pid="$tiny_pid"
if [ -z "$port" ]; then
    printf 'SKIP chat descriptor leak: the server would not start at ulimit -n 6 on this host\n' >&2
else
    # This send is expected to fail; what matters is what the server does.
    send_one "$port" tiny '#fd' 'this one cannot be served'
    sleep 1
    # Not `kill -0`: this process is our own child, so when it aborts it stays
    # an unreaped zombie and `kill -0` still succeeds. The process state is what
    # distinguishes "running" from "dead but not yet reaped".
    srv_state="$(ps -o state= -p "$tiny_pid" 2>/dev/null | tr -d ' ')"
    case "$srv_state" in
        ''|Z*)
            t_fail "the server died when it ran out of descriptors instead of refusing the connection (state '${srv_state:-gone}'): $(tail -2 "$work/tiny.err" 2>/dev/null)" ;;
    esac
    case "$(cat "$work/tiny.err" 2>/dev/null)" in
        *'cannot accept connections'*) : ;;
        *) t_fail "the server never reported that it could not accept connections; stderr was: $(cat "$work/tiny.err" 2>/dev/null)" ;;
    esac
    # Only "panicked": an EMFILE that is merely *reported* is the correct
    # behaviour being asserted two checks above, and matching on the error text
    # itself would call that a panic.
    case "$(cat "$work/tiny.err" 2>/dev/null)" in
        *panicked*)
            t_fail "the server panicked on descriptor exhaustion instead of reporting it: $(cat "$work/tiny.err")" ;;
    esac
fi

t_end
