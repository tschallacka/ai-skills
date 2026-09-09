#!/usr/bin/env bash
# MODE: DEV
# test-chat-owner-socket.sh - one client session owns the connection (T107).
#
# A running `tail` binds a control socket in the session state dir and serves
# the other verbs on ITS connection. What that buys is B283 closed by
# construction: a send no longer registers a second time under the same nick,
# so the server has no collision to suffix, and the message arrives from the
# nick the agent chose.
#
# Every assertion reads the CHANNEL LOG -- the bytes the server persisted --
# rather than the sender's own output. A report built from the input argument is
# not evidence, which is how B266 stayed invisible for as long as it did.
#
# The suffix assertion carries its own positive control: `--no-session` is
# documented to bypass the socket, so the same send made that way must STILL
# arrive suffixed. Without that half, a test asserting "not suffixed" would also
# pass on a build where the server had simply stopped suffixing anything.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo="$(cd "$root/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$repo/planning/tests" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/chat-owner.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

SERVER="$repo/target/release/chat-server-rs"
CLIENT="$repo/target/release/chat-client-rs"

# A missing cargo AND missing prebuilt binaries is a loud SKIP, not a failure:
# a host may ship only the release artifacts. Same ladder as test-chat.sh.
if ! command -v cargo >/dev/null 2>&1; then
    prebuilt_server="$(ls "$root"/bin/*/chat-server-rs 2>/dev/null | head -1 || true)"
    prebuilt_client="$(ls "$root"/bin/*/chat-client-rs 2>/dev/null | head -1 || true)"
    if [ -n "$prebuilt_server" ] && [ -n "$prebuilt_client" ]; then
        SERVER="$prebuilt_server"
        CLIENT="$prebuilt_client"
    else
        t_skip 'chat owner socket: no cargo and no prebuilt chat/bin binaries'
    fi
else
    ( cd "$repo/src/chat-server-rs" && cargo build --release >/dev/null 2>&1 ) \
        || t_fail "cargo build chat-server-rs failed"
    ( cd "$repo/src/chat-client-rs" && cargo build --release >/dev/null 2>&1 ) \
        || t_fail "cargo build chat-client-rs failed"
fi

# A unix socket address is a fixed-size buffer, so the state home goes under
# T_SOCKET_TMPDIR -- /tmp/s.XXXXX, deliberately not nested inside TMPDIR. Only
# the socket-bearing home lives there; logs and server output stay in TMPDIR.
home="${T_SOCKET_TMPDIR:-$temporary_root}/h"
mkdir -p "$home"

chan='#owned'
nick='owner'
key='t107'
# The RECORD is at a path this test can derive; the SOCKET is not, and must not
# be guessed. The owner puts the socket in the user's runtime directory when
# there is one and beside the state otherwise, and it writes the path it chose
# into the record. Reading it from there is both the robust thing and the
# contract under test: a borrower finds the socket the same way.
record="$home/owners/$key.json"
owner_socket() {
    [ -f "$record" ] || return 0
    rjq -r '.socket // empty' "$record" 2>/dev/null
}
# Wait for a bound socket, and return its path. Empty means the owner never
# came up, which each caller judges for itself.
await_socket() {
    local waited=0 path=""
    while [ "$waited" -lt 50 ]; do
        path="$(owner_socket)"
        if [ -n "$path" ] && [ -S "$path" ]; then
            printf '%s\n' "$path"
            return 0
        fi
        waited=$((waited + 1))
        sleep 0.2
    done
    printf '%s\n' "$path"
}

AI_CHAT_HOME="$home" CHAT_ANNOUNCE=0 \
    "$SERVER" 0 >"$temporary_root/server.out" 2>"$temporary_root/server.err" &
server_pid=$!
cleanup() {
    kill "$server_pid" 2>/dev/null || true
    # Reaped, not just signalled: without this bash 3.2 prints its own job
    # notification ("Terminated") to the test log after the PASS line, which
    # reads like a failure in a CI log and is not one.
    wait "$server_pid" 2>/dev/null || true
    [ -n "${tail_pid:-}" ] && kill "$tail_pid" 2>/dev/null || true
    rm -rf "$temporary_root"
}
trap cleanup EXIT

port=""
for _ in $(seq 1 40); do
    [ -s "$home/server.port" ] && { port="$(cat "$home/server.port")"; break; }
    sleep 0.2
done
case "$port" in ''|*[!0-9]*) t_fail "server did not report a port"; t_end; exit 1 ;; esac

client() { AI_CHAT_HOME="$home" "$CLIENT" --session "$key" "$@" --insecure; }

# A DIFFERENT agent posting into a channel. Needed wherever an assertion reads
# the tail's own output: the server deliberately does not echo a PRIVMSG back
# to its sender (B249 -- an echo rendered every message twice in a standard
# client), so the owner never sees its own posts and a test that sent them to
# itself would be asserting on traffic that is never pushed. `--no-session` so
# it bypasses the control socket, and its own nick so it cannot collide.
peer_send() { # <chan> <text>
    AI_CHAT_HOME="$home" "$CLIENT" send --server 127.0.0.1:"$port" --nick peer \
        --chan "$1" --text "$2" --no-session --insecure
}

client session set --server 127.0.0.1:"$port" --nick "$nick" >/dev/null
client join --chan "$chan" >/dev/null

# ── the tail takes ownership ────────────────────────────────────────────────
client tail --chan "$chan" --presence >"$temporary_root/tail.out" 2>"$temporary_root/tail.err" &
tail_pid=$!

socket="$(await_socket)"
[ -n "$socket" ] && [ -S "$socket" ] \
    || t_fail "the tail bound no control socket (record: $(cat "$record" 2>/dev/null)): $(cat "$temporary_root/tail.err")"

t_assert_eq 'the record directory is private' \
    "$(ls -ld "$home/owners" | cut -c2-10)" 'rwx------'
t_assert_eq 'the socket directory is private' \
    "$(ls -ld "$(dirname "$socket")" | cut -c2-10)" 'rwx------'
t_assert_eq 'the socket is owner-only' \
    "$(ls -l "$socket" | cut -c2-10)" 'rw-------'
# The socket must be somewhere short by construction. A path near the address
# limit is how a tail silently declines to own anything at all.
t_assert_eq 'the socket path is well inside the address limit' \
    "$([ "${#socket}" -lt 100 ] && echo inside || echo "over:${#socket}")" 'inside'

# ── a forwarded send is not suffixed, which is B283 ─────────────────────────
client send --chan "$chan" --text 'through the owner' >"$temporary_root/send.out" 2>&1 \
    || t_fail "a forwarded send failed: $(cat "$temporary_root/send.out")"

log="$home/channels/$chan.log"
sender_of() { # <text-fragment>
    awk -v want="$1" '$0 ~ want { print $5 }' "$log" | tail -1
}
t_assert_eq 'a send through the owner arrives from the unsuffixed nick' \
    "$(sender_of 'through the owner')" "$nick"

# The positive control. --no-session bypasses the socket by design, so this send
# registers a second time under a nick the tail is already holding, and the
# server suffixes it. If this stops being suffixed the assertion above proves
# nothing, and the test says which of the two changed.
AI_CHAT_HOME="$home" "$CLIENT" send --server 127.0.0.1:"$port" --nick "$nick" \
    --chan "$chan" --text 'around the owner' --no-session --insecure \
    >"$temporary_root/bypass.out" 2>&1 \
    || t_fail "the bypassing send failed: $(cat "$temporary_root/bypass.out")"
t_assert_eq 'a send that bypasses the owner is still suffixed by the server' \
    "$(sender_of 'around the owner')" "$nick-2"

# ── names is served on the same connection ─────────────────────────────────
# One entry, not two: the borrowed query does not add a member, and the tail's
# own membership is not duplicated by it.
t_assert_eq 'names served by the owner lists the tailing nick exactly once' \
    "$(client names --chan "$chan" 2>&1 | awk -v n="$nick" '$0 == n' | wc -l | tr -d ' ')" '1'

# ── read is served, and reports what the wire holds ───────────────────────
t_assert_eq 'read served by the owner returns the persisted messages' \
    "$(client read --chan "$chan" --since 0 2>&1 | awk '/through the owner|around the owner/' | wc -l | tr -d ' ')" '2'

# ── a forwarded join is FOLLOWED, not just joined (B296) ──────────────────
# The JOIN goes out on the owner's connection, so the server counts it as a
# member and relays that channel's traffic here. Before T112 the tail filtered
# on its single channel and dropped every one of those lines: the agent sat in
# a member list it could not hear. So the assertion is on the TAIL'S OWN
# OUTPUT -- the only place that distinguishes following from merely joining.
second='#alsoowned'
client join --chan "$second" >"$temporary_root/join2.out" 2>&1 \
    || t_fail "a forwarded join failed: $(cat "$temporary_root/join2.out")"
t_assert_eq 'a forwarded join reports the whole followed set' \
    "$(awk '/following .*'"${chan#\#}"'.*'"${second#\#}"'/ {found=1} END {print found+0}' "$temporary_root/join2.out")" '1'

peer_send "$second" 'into the joined channel' >/dev/null 2>&1 \
    || t_fail "a peer send to the newly joined channel failed"
followed=0
for _ in $(seq 1 30); do
    if awk '/into the joined channel/ {found=1} END {exit !found}' "$temporary_root/tail.out"; then
        followed=1
        break
    fi
    sleep 0.2
done
t_assert_eq 'the tail prints traffic from a channel joined after it started' \
    "$followed" '1'

# ── leave drops one channel and keeps the tail on the rest ────────────────
client leave --chan "$second" >"$temporary_root/leave2.out" 2>&1 \
    || t_fail "leave on a followed channel failed: $(cat "$temporary_root/leave2.out")"
t_assert_eq 'leave says what is still followed' \
    "$(awk '/still following/ {found=1} END {print found+0}' "$temporary_root/leave2.out")" '1'
kill -0 "$tail_pid" 2>/dev/null \
    || t_fail "leaving one of two channels stopped the tail"
[ -S "$socket" ] || t_fail "leaving one of two channels took the control socket down"

# ── leaving the LAST channel stops the tail ───────────────────────────────
# Tschallacka's ruling (T112): a leave means leave. A tail following nothing
# would otherwise hold a connection subscribed to no channel while still
# answering as the session's owner -- present nowhere, waking on nothing, and
# looking alive to anything that reads the socket.
client leave --chan "$chan" >"$temporary_root/leave.out" 2>&1 \
    || t_fail "leave on the last channel failed: $(cat "$temporary_root/leave.out")"
t_assert_eq 'leave on the last channel says the tail is stopping' \
    "$(awk '/the tail is stopping/ {found=1} END {print found+0}' "$temporary_root/leave.out")" '1'
stopped=0
for _ in $(seq 1 40); do
    kill -0 "$tail_pid" 2>/dev/null || { stopped=1; break; }
    sleep 0.25
done
t_assert_eq 'and the tail actually exits' "$stopped" '1'
# Killed before it is waited on: waiting on a process already reported as stuck
# turns a failing test into a hanging one, and a hang carries no name.
wait "$tail_pid" 2>/dev/null || true
t_assert_eq 'and its control socket is gone, so callers fall back' \
    "$([ -S "$socket" ] && echo present || echo gone)" 'gone'

# ── with no owner at all, every verb works as it did before T107 ─────────
tail_pid=""
client send --chan "$chan" --text 'after the owner' >"$temporary_root/after.out" 2>&1 \
    || t_fail "a send with no owner failed: $(cat "$temporary_root/after.out")"
t_assert_eq 'a send with no owner running still reaches the channel' \
    "$(sender_of 'after the owner')" "$nick"

# ── a tail can be started on several channels at once ────────────────────
# `--chan` is repeatable. Asserted from the tail's own output again, one
# message per channel: a tail that silently followed only the first would
# still bind the socket and still answer every verb, so nothing else here
# would notice.
client tail --chan "$chan" --chan "$second" \
    >"$temporary_root/tail2.out" 2>"$temporary_root/tail2.err" &
tail_pid=$!
for _ in $(seq 1 50); do
    [ -S "$socket" ] && break
    sleep 0.2
done
t_assert_eq 'a later tail takes the socket the stopped one released' \
    "$([ -S "$socket" ] && echo yes || echo no)" 'yes'

peer_send "$chan" 'first of two' >/dev/null 2>&1 \
    || t_fail "a peer send to the first followed channel failed"
peer_send "$second" 'second of two' >/dev/null 2>&1 \
    || t_fail "a peer send to the second followed channel failed"
both=0
for _ in $(seq 1 40); do
    if awk '/first of two/ {a=1} /second of two/ {b=1} END {exit !(a && b)}' \
        "$temporary_root/tail2.out"; then
        both=1
        break
    fi
    sleep 0.25
done
t_assert_eq 'a tail started with two --chan prints traffic from both' "$both" '1'

# ── a CRASHED owner leaves its socket, and callers still fall back ───────
# Killed rather than asked to leave, so nothing runs to unlink the path and the
# next verb finds a socket with nobody behind it. It must reconnect on its own
# instead of waiting out a deadline: this is the half that keeps one crash from
# taking the session's other verbs down with it.
kill "$tail_pid" 2>/dev/null || true
wait "$tail_pid" 2>/dev/null || true
tail_pid=""
client send --chan "$chan" --text 'after the crash' >"$temporary_root/crash.out" 2>&1 \
    || t_fail "a send after the owner was killed failed: $(cat "$temporary_root/crash.out")"
t_assert_eq 'a send after a crashed owner still reaches the channel' \
    "$(sender_of 'after the crash')" "$nick"

t_end
