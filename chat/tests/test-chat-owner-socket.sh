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
        printf 'SKIP chat owner socket: no cargo and no prebuilt chat/bin binaries\n' >&2
        t_end
        exit 0
    fi
else
    ( cd "$repo/src/chat-server-rs" && cargo build --release >/dev/null 2>&1 ) \
        || t_fail "cargo build chat-server-rs failed"
    ( cd "$repo/src/chat-client-rs" && cargo build --release >/dev/null 2>&1 ) \
        || t_fail "cargo build chat-client-rs failed"
fi

# A unix socket address is a fixed-size buffer -- 104 bytes on macOS -- and the
# owner declines to bind a path over that rather than binding a truncated one.
# A long path would therefore make every assertion below test the FALLBACK path
# while claiming to test the socket.
#
# So the state home goes under lib-test.sh's T_SOCKET_TMPDIR, which exists for
# exactly this: /tmp/s.XXXXX, twelve characters, deliberately not nested inside
# TMPDIR. Measured here -- under `bash32-run`, where nix's TMPDIR and the
# suite's own scratch nest, $TMPDIR reached 103 characters and this test skipped
# itself while reporting PASS. Only the socket-bearing home lives there; the
# server's output and the logs this test reads stay under TMPDIR.
home="${T_SOCKET_TMPDIR:-$temporary_root}/h"
mkdir -p "$home"

chan='#owned'
nick='owner'
key='t107'
socket="$home/owners/$key.sock"

# Measured on the address that will actually be bound, not on its parent: the
# owner's own limit is on the whole path, and a guard on the directory either
# skips runs that would have worked or admits ones that cannot bind.
if [ "${#socket}" -ge 100 ]; then
    printf 'SKIP chat owner socket: %s is too long for a socket address (%s chars)\n' \
        "$socket" "${#socket}" >&2
    t_end
    exit 0
fi

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

client session set --server 127.0.0.1:"$port" --nick "$nick" >/dev/null
client join --chan "$chan" >/dev/null

# ── the tail takes ownership ────────────────────────────────────────────────
client tail --chan "$chan" --presence >"$temporary_root/tail.out" 2>"$temporary_root/tail.err" &
tail_pid=$!

for _ in $(seq 1 50); do
    [ -S "$socket" ] && break
    sleep 0.2
done
[ -S "$socket" ] || t_fail "the tail did not bind a control socket at $socket: $(cat "$temporary_root/tail.err")"

t_assert_eq 'the owner directory is private' \
    "$(ls -ld "$home/owners" | cut -c2-10)" 'rwx------'
t_assert_eq 'the socket is owner-only' \
    "$(ls -l "$socket" | cut -c2-10)" 'rw-------'

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

# ── leaving the channel the tail holds is refused, not half-done ──────────
# Parting it would leave the tail running and deaf while still holding the
# connection every other verb borrows.
rc=0
client leave --chan "$chan" >"$temporary_root/leave.out" 2>&1 || rc=$?
t_assert_eq 'leave on the tailed channel is refused' "$rc" '64'
t_assert_eq 'and it says stopping the tail is what to do' \
    "$(awk '/stop the tail/ {found=1} END {print found+0}' "$temporary_root/leave.out")" '1'

# ── the socket goes away with the tail, so callers fall back ──────────────
kill "$tail_pid" 2>/dev/null || true
wait "$tail_pid" 2>/dev/null || true
tail_pid=""

# A killed owner cannot unlink anything, so the path may outlive it. What must
# hold is that a verb finding it does NOT hang: it reconnects on its own. This
# is the fallback contract, and it is the half that keeps a crash from taking
# the session's other verbs down with it.
client send --chan "$chan" --text 'after the owner' >"$temporary_root/after.out" 2>&1 \
    || t_fail "a send after the owner died failed: $(cat "$temporary_root/after.out")"
t_assert_eq 'a send after the owner is gone still reaches the channel' \
    "$(sender_of 'after the owner')" "$nick"

# And the next tail reclaims the path rather than being locked out by it.
client tail --chan "$chan" >"$temporary_root/tail2.out" 2>"$temporary_root/tail2.err" &
tail_pid=$!
for _ in $(seq 1 50); do
    [ -S "$socket" ] && break
    sleep 0.2
done
t_assert_eq 'a second tail reclaims the socket a dead owner left' \
    "$([ -S "$socket" ] && echo yes || echo no)" 'yes'
kill "$tail_pid" 2>/dev/null || true
wait "$tail_pid" 2>/dev/null || true
tail_pid=""

t_end
