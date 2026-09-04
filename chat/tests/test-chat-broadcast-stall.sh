#!/usr/bin/env bash
# MODE: DEV
# test-chat-broadcast-stall.sh - one idle subscriber must not wedge the server.
#
# B122. The broadcast path used to hold `hub.writers` while locking a second
# connection's state and writing to its socket. `hub.writers` is the lock the
# accept loop takes to register a new connection, so while a broadcaster was
# stuck on a subscriber that was not reading, connects completed through the
# kernel backlog and were then never serviced. Measured on the unfixed server:
# ONE attached `tail` subscriber, and the next two sends both failed with
# `send: Resource temporarily unavailable (os error 11)` after ten seconds
# each - including a send from a fresh client to a different channel - with
# neither message appended to any log.
#
# The assertions below are therefore about the server making progress for
# everyone else while one subscriber is not consuming: a fresh connection is
# serviced, later sends land, and a subscriber that is alive but stopped
# (SIGSTOP - the case that does not recover on its own, since the server never
# learns the peer is gone) does not stall the rest either.
#
# To watch every assertion fail, point it at a server built before the fix:
#   CHAT_SERVER_BIN=/path/to/old/chat-server-rs chat/tests/test-chat-broadcast-stall.sh
#
# A missing cargo or missing rust binaries is a loud SKIP, not a failure.

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
    printf 'SKIP chat broadcast stall: no timeout(1) - the wedge assertions need a bounded wait\n' >&2
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
        printf 'SKIP chat broadcast stall: no cargo and no prebuilt chat/bin binaries\n' >&2
        t_end
        exit 0
    fi
else
    ( cd "$repo/src/chat-server-rs" && cargo build --release >/dev/null 2>&1 ) \
        || t_fail "cargo build chat-server-rs failed"
    ( cd "$repo/src/chat-client-rs" && cargo build --release >/dev/null 2>&1 ) \
        || t_fail "cargo build chat-client-rs failed"
fi
# The mutation hook: an older server binary, to show the assertions bite.
SERVER="${CHAT_SERVER_BIN:-$SERVER}"

work="$(mktemp -d "${TMPDIR:-/tmp}/chat-stall.XXXXXX")"
home="$work/home"
mkdir -p "$home"
subscriber_pid=""
server_pid=""
cleanup() {
    # CONT first: a stopped process cannot act on a TERM until it is resumed.
    [ -n "$subscriber_pid" ] && kill -CONT "$subscriber_pid" 2>/dev/null
    [ -n "$subscriber_pid" ] && kill "$subscriber_pid" 2>/dev/null
    [ -n "$server_pid" ] && kill "$server_pid" 2>/dev/null
    rm -rf "$work"
}
trap cleanup EXIT

AI_CHAT_HOME="$home" "$SERVER" 0 >"$work/server.out" 2>"$work/server.err" &
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

# Each client gets its own state dir: the cert pin and the session cursor are
# per-directory, and a shared one would make one client's session decide
# another's server.
send_as() { # <nick> <chan> <text> <seconds> -> 0 when the send completed
    local nick="$1" chan="$2" text="$3" limit="$4"
    local dir="$work/c-$nick"
    mkdir -p "$dir"
    AI_CHAT_HOME="$dir" timeout "$limit" "$CLIENT" send \
        --server 127.0.0.1:"$port" --nick "$nick" --chan "$chan" --text "$text" \
        --insecure --no-session >"$work/send-$nick.out" 2>&1
}

logged() { # <chan> <text> -> 0 when the channel log carries that message
    local chan="$1" text="$2"
    [ -f "$home/channels/$chan.log" ] || return 1
    case "$(cat "$home/channels/$chan.log")" in
        *"$text"*) return 0 ;;
        *) return 1 ;;
    esac
}

# One subscriber, attached the way an agent attaches: `tail` JOINs the channel
# and then sleeps between history polls, so it is a live connection that is not
# reading for seconds at a time. That is the whole fault injection - no flood,
# no crafted client.
#
# Membership is not what makes it bite, and neither is the subscriber consuming
# broadcasts. The unfixed broadcast path locked every other connection's state
# before checking whether that connection was even in the channel, so any
# attached connection starves it; and `tail` reads history with FETCH and
# ignores PRIVMSG lines outright, so it never consumes a broadcast at all. What
# it takes is a connection whose own thread is holding its state.
mkdir -p "$work/c-sub"
AI_CHAT_HOME="$work/c-sub" "$CLIENT" tail \
    --server 127.0.0.1:"$port" --nick stallsub --chan '#stall' \
    --insecure --no-session >"$work/sub.out" 2>&1 &
subscriber_pid=$!

# Precondition, asserted rather than slept for: the subscriber is connected and
# being serviced, proven by it printing a message another client sent. Without
# this the whole test could pass vacuously against a subscriber that never got
# off the ground - there would be nothing holding a connection state, and every
# assertion below would be trivially true.
#
# It does NOT prove broadcast delivery: `tail` gets that message from its FETCH
# poll of the channel log. Nothing the shipped client can do asserts the
# broadcast path end to end, because it discards PRIVMSG lines; the outbox unit
# tests in src/chat-server-rs/src/main.rs cover the queueing itself.
send_as opener '#stall' 'subscriber-armed' 30
attached=false
for _ in $(seq 1 60); do
    case "$(cat "$work/sub.out" 2>/dev/null)" in
        *subscriber-armed*) attached=true; break ;;
    esac
    sleep 0.5
done
if [ "$attached" != true ]; then
    t_fail "the subscriber never received a message, so it was not a connection the server was servicing: $(cat "$work/sub.out" 2>/dev/null)"
    t_end
    exit 1
fi

# 1. A FRESH connection must still be serviced. This is the accept-loop
#    assertion: the connect completes through the kernel backlog either way, so
#    what is being tested is whether the server ever registers and answers it.
#    On the unfixed server this failed with os error 11 after ten seconds.
if send_as newcomer '#other' 'a-fresh-connection' 15; then
    logged '#other' 'a-fresh-connection' \
        || t_fail "the fresh client's send returned but the message never reached the log"
else
    t_fail "a fresh connection was not serviced while one subscriber was attached: $(cat "$work/send-newcomer.out")"
fi

# 2. And it is not merely slow: three consecutive sends must all land. A wedge
#    that clears after one message would still pass assertion 1 by luck.
i=1
while [ "$i" -le 3 ]; do
    if send_as "sender$i" '#stall' "consecutive-$i" 15; then
        logged '#stall' "consecutive-$i" \
            || t_fail "send $i returned but message consecutive-$i never reached the log"
    else
        t_fail "send $i stalled while one subscriber was attached: $(cat "$work/send-sender$i.out")"
    fi
    i=$((i + 1))
done

# 3. A subscriber that is alive but not reading at all. SIGSTOP is the case with
#    no self-recovery: the peer never closes its socket, so nothing tells the
#    server it is gone, and on the unfixed server the only way out was a
#    restart. The rest of the server must keep working regardless.
kill -STOP "$subscriber_pid" 2>/dev/null || t_fail "could not stop the subscriber"
if send_as afterstop '#stall' 'after-the-subscriber-stopped' 15; then
    logged '#stall' 'after-the-subscriber-stopped' \
        || t_fail "the send after SIGSTOP returned but never reached the log"
else
    t_fail "a stopped (alive, non-reading) subscriber stalled the server: $(cat "$work/send-afterstop.out")"
fi

# 4. Recovery must not need a restart: with the stopped subscriber still there,
#    the server keeps taking new connections on other channels too.
if send_as afterstop2 '#other' 'still-serving-other-channels' 15; then
    logged '#other' 'still-serving-other-channels' \
        || t_fail "the second send after SIGSTOP returned but never reached the log"
else
    t_fail "the server stopped serving other channels while a subscriber was stopped: $(cat "$work/send-afterstop2.out")"
fi

kill -CONT "$subscriber_pid" 2>/dev/null
kill "$subscriber_pid" 2>/dev/null
subscriber_pid=""

# 5. Delivery still works. The queue replaced a direct socket write, so this is
#    the regression the redesign could cause, and no shipped client can catch
#    it: `chat-client-rs` reads history with FETCH and discards PRIVMSG lines.
#    A stock TLS IRC client is what consumes a broadcast, so the watcher here is
#    openssl s_client speaking the wire format by hand. Its stdin is a fifo held
#    open by a sleeping writer, because s_client exits when stdin closes.
if command -v openssl >/dev/null 2>&1 && command -v mkfifo >/dev/null 2>&1; then
    mkfifo "$work/watch.in"
    (
        printf 'NICK watcher\r\nUSER watcher 0 * :watcher\r\nJOIN #stall\r\n'
        sleep 25
    ) >"$work/watch.in" &
    watch_feeder=$!
    timeout 25 openssl s_client -quiet -verify_quiet \
        -connect 127.0.0.1:"$port" -servername localhost \
        <"$work/watch.in" >"$work/watch.out" 2>/dev/null &
    watch_pid=$!
    # The watcher must be joined before the message is sent: a broadcast is not
    # replayed, so a late JOIN would make this assert nothing.
    joined=false
    for _ in $(seq 1 40); do
        case "$(cat "$work/watch.out" 2>/dev/null)" in
            *'End of /NAMES list'*) joined=true; break ;;
        esac
        sleep 0.5
    done
    if [ "$joined" != true ]; then
        t_fail "the openssl watcher never completed a JOIN, so broadcast delivery was not tested: $(cat "$work/watch.out" 2>/dev/null)"
    else
        send_as broadcaster '#stall' 'delivered-by-broadcast' 15 \
            || t_fail "the send to the joined watcher stalled: $(cat "$work/send-broadcaster.out")"
        delivered=false
        for _ in $(seq 1 40); do
            case "$(cat "$work/watch.out" 2>/dev/null)" in
                *delivered-by-broadcast*) delivered=true; break ;;
            esac
            sleep 0.5
        done
        [ "$delivered" = true ] \
            || t_fail "a joined watcher never received the broadcast: $(cat "$work/watch.out")"
    fi
    kill "$watch_pid" "$watch_feeder" 2>/dev/null
else
    printf 'SKIP chat broadcast stall: no openssl or mkfifo - broadcast delivery was not asserted\n' >&2
fi

# 6. Nothing above may have been bought with a panicked server thread: a
#    poisoned mutex would show up here, and the process must still be alive.
kill -0 "$server_pid" 2>/dev/null || t_fail "the server process died during the run"
case "$(cat "$work/server.err" 2>/dev/null)" in
    *panicked*) t_fail "a server thread panicked: $(cat "$work/server.err")" ;;
esac

t_end
