#!/usr/bin/env bash
# MODE: DEV
# test-chat-resolution.sh - the server's port session-preference, the beacon's
# connectable host, and the client's server-resolution ladder.
#
# Three contracts from the LAN-chat work:
#   1. the server prefers the port recorded in server.port on every later
#      start (the file is the session config), an explicit argv port
#      overrides, and a taken session port falls back to ephemeral;
#   2. the announce beacon carries a connectable host (CHAT_ANNOUNCE_HOST,
#      then the primary routable address) - never the useless literal
#      localhost - and the name follows it;
#   3. the client resolves its server by ladder: explicit --server, then the
#      session (probed), then the last-discovered cache, then a fresh UDP
#      pass - a dead session server heals to whatever answers, and the heal
#      is written back to the session and the cache.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo="$(cd "$root/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$repo/planning/tests" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C
SERVER="$repo/src/chat-server-rs/target/release/chat-server-rs"
CLIENT="$repo/src/chat-client-rs/target/release/chat-client-rs"

if ! command -v cargo >/dev/null 2>&1; then
    prebuilt_server="$(ls "$root"/bin/*/chat-server-rs 2>/dev/null | head -1 || true)"
    prebuilt_client="$(ls "$root"/bin/*/chat-client-rs 2>/dev/null | head -1 || true)"
    if [ -n "$prebuilt_server" ] && [ -n "$prebuilt_client" ]; then
        SERVER="$prebuilt_server"
        CLIENT="$prebuilt_client"
    else
        printf 'SKIP chat-resolution: no cargo and no prebuilt chat/bin binaries\n' >&2
        t_end
        exit 0
    fi
fi

temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/chat-resolution.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT
pids=()
stop_server() { [ -n "${1:-}" ] && kill "$1" 2>/dev/null && wait "$1" 2>/dev/null || true; }

start_server() { # <home> [extra env as VAR=val pairs already set by caller]
    local h="$1"
    AI_CHAT_HOME="$h" "$SERVER" >"$h/server.out" 2>"$h/server.err" &
    pids+=($!)
}

wait_port() { # <home> -> prints the recorded port
    local h="$1" p=""
    for _ in $(seq 1 40); do
        [ -s "$h/server.port" ] && { p="$(cat "$h/server.port")"; break; }
        sleep 0.2
    done
    printf '%s' "$p"
}

# ---- 1. the session port is preferred across restarts ----------------------
h1="$temporary_root/h1"
mkdir -p "$h1"
start_server "$h1"
p1="$(wait_port "$h1")"
case "$p1" in ''|*[!0-9]*) t_fail "first server did not report a port"; t_end; exit 1 ;; esac
stop_server "${pids[-1]}"

start_server "$h1"
p1b="$(wait_port "$h1")"
[ "$p1b" = "$p1" ] || t_fail "restart did not prefer the session port: got $p1b, recorded $p1"

# ---- 2. an explicit argv port overrides the session ------------------------
override=$((p1 + 1))
AI_CHAT_HOME="$h1" "$SERVER" "$override" >"$h1/s3.out" 2>"$h1/s3.err" &
pids+=($!)
sleep 0.5
p3="$(wait_port "$h1")"
[ "$p3" = "$override" ] || t_fail "explicit port was overridden: got $p3, wanted $override"

# ---- 3. a taken session port falls back to ephemeral -----------------------
# S3 holds $override (the recorded session port); a new server must say so
# and land somewhere else.
start_server "$h1"
p4="$(wait_port "$h1")"
[ "$p4" != "$override" ] || t_fail "the taken session port was reused"
grep -q "taken" "$h1/server.err" || t_fail "the taken-port fallback was not announced: $(cat "$h1/server.err" | head -1)"

# ---- 4. the beacon carries a connectable host, never bare localhost --------
bh="$temporary_root/beacon-home"
mkdir -p "$bh"
AI_CHAT_HOME="$bh" CHAT_ANNOUNCE=1 CHAT_BCAST=127.0.0.1 CHAT_BEACON_PORT=47995 \
    CHAT_ANNOUNCE_HOST=203.0.113.7 "$SERVER" >"$bh/server.out" 2>"$bh/server.err" &
pids+=($!)
bport="$(wait_port "$bh")"
case "$bport" in ''|*[!0-9]*) t_fail "beacon server did not report a port"; t_end; exit 1 ;; esac
ccli() { # <dir> <args...> -> the client with its own state dir
    local d="$temporary_root/$1"; shift
    mkdir -p "$d"
    AI_CHAT_HOME="$d" timeout 8 "$CLIENT" "$@"
}
disco="$(ccli cdisc discover --bcast 127.0.0.1 --beacon-port 47995 --wait 3 --json 2>/dev/null || true)"
case "$disco" in
    *'"host":"203.0.113.7"'*) : ;;
    *) t_fail "the beacon did not carry the announced host: $disco" ;;
esac
case "$disco" in
    *'"name":"ai-chat/203.0.113.7"'*) : ;;
    *) t_fail "the server name did not follow the announced host: $disco" ;;
esac
case "$disco" in
    *'localhost'*) t_fail "the beacon still announces localhost: $disco" ;;
    *) : ;;
esac
disco_port="$(printf '%s' "$disco" | grep -oE '"port":[0-9]+' | head -1 | cut -d: -f2 || true)"
[ "$disco_port" = "$bport" ] || t_fail "the beacon port did not parse: $disco"

# ---- 5. the ladder: dead session -> discovery -> healed session ------------
# A live, announcing server on loopback (its beacon host is 127.0.0.1, which
# the ladder can actually dial), a client session pointing at a dead port,
# and an empty cache: read must resolve via discovery and heal the session.
lh="$temporary_root/ladder-home"
mkdir -p "$lh"
# Default beacon port, loopback broadcast: the caller does nothing. The
# client resolves the dead session through discovery with no env and no
# flags, which is the contract this suite pins.
AI_CHAT_HOME="$lh" CHAT_ANNOUNCE=1 CHAT_BCAST=127.0.0.1 \
    CHAT_ANNOUNCE_HOST=127.0.0.1 "$SERVER" >"$lh/server.out" 2>"$lh/server.err" &
pids+=($!)
lport="$(wait_port "$lh")"
case "$lport" in ''|*[!0-9]*) t_fail "ladder server did not report a port"; t_end; exit 1 ;; esac

cl="$temporary_root/client"
mkdir -p "$cl"
c_session() { AI_CHAT_HOME="$cl" timeout 8 "$CLIENT" "$@"; }
c_session session set --server 127.0.0.1:1 --nick junkbox >/dev/null 2>&1 || {
    t_fail "session set failed"
}
c_session send --chan '#ladder' --text 'resolve me' >/dev/null 2>&1 || {
    t_fail "send did not resolve a dead session to the live server"
}
# The healed address is whichever server answered the discovery ladder - on a
# CI runner that is this test's loopback server; on a box with a live LAN
# server it is the LAN one, which the ladder is right to prefer. Either way
# it must be alive, not the dead session address.
healed="$(c_session session show | sed -n 's/^server=//p')"
[ "$healed" != "127.0.0.1:1" ] || t_fail "the session did not heal away from the dead address: $healed"
c_session read --server "$healed" --nick junkbox --chan '#ladder' --since 0 >/dev/null 2>&1 \
    || t_fail "the healed session address is not alive: $healed"
grep -q "$healed" "$cl/discovered-servers.txt" \
    || t_fail "the resolved server was not cached"

# The cache is the fast track: with the session pointed at a dead port again,
# the cached address answers before any beacon is needed.
c_session session set --server 127.0.0.1:1 --nick junkbox >/dev/null 2>&1
c_session read --chan '#ladder' --since 0 >/dev/null 2>&1 \
    || t_fail "the cache fast-track did not resolve the live server"
healed2="$(c_session session show | sed -n 's/^server=//p')"
[ "$healed2" = "$healed" ] || t_fail "the cache hit did not heal the session: $healed2"

# An explicit --server wins and is used as-is: the dead address surfaces in
# the connect error rather than being silently replaced.
err="$(c_session read --server 127.0.0.1:1 --nick junkbox --chan '#ladder' --since 0 2>&1 || true)"
case "$err" in
    *'127.0.0.1:1'*) : ;;
    *) t_fail "an explicit --server did not win: $err" ;;
esac

for pid in "${pids[@]}"; do stop_server "$pid"; done
t_end
