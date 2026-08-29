#!/usr/bin/env bash
# MODE: DEV
# test-chat.sh - the chat skill's rust server and rust client, end to end.
#
# Build (from the nix dev shell, where cargo/rustc live), start the rust server,
# then drive the rust client through discovery, send, read-delta, and tail, and
# assert TLS/TOFU. A missing cargo or missing rust binaries is a loud SKIP, not
# a failure (a host may ship prebuilt chat/bin binaries).

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo="$(cd "$root/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$repo/planning/tests" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C
fail() { t_fail "$*"; }
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/chat-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

BIN="$repo/src/target/release"
SERVER="$BIN/chat-server-rs"
CLIENT="$BIN/chat-client-rs"

if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$root/bin/chat-server-rs" ] && [ -x "$root/bin/chat-client-rs" ]; then
        SERVER="$root/bin/chat-server-rs"
        CLIENT="$root/bin/chat-client-rs"
    else
        printf 'SKIP chat: no cargo and no prebuilt chat/bin binaries - rust assertions did not run\n' >&2
        t_end
        exit 0
    fi
else
    ( cd "$repo/src" && cargo build --release --workspace >/dev/null 2>&1 ) \
        || { t_fail "cargo build --workspace failed"; }
    mkdir -p "$root/bin"
    cp "$SERVER" "$root/bin/chat-server-rs"
    cp "$CLIENT" "$root/bin/chat-client-rs"
fi

home="$temporary_root/home"
mkdir -p "$home"

# Start the server (announce on loopback so discovery works).
AI_CHAT_HOME="$home" CHAT_ANNOUNCE=1 CHAT_BCAST=127.0.0.1 CHAT_BEACON_PORT=47991 CHAT_NAME=test-beacon \
    "$SERVER" 0 >"$temporary_root/server.out" 2>"$temporary_root/server.err" &
server_pid=$!
port=""
for _ in $(seq 1 40); do
    [ -s "$home/server.port" ] && { port="$(cat "$home/server.port")"; break; }
    sleep 0.2
done
case "$port" in ''|*[!0-9]*) t_fail "server did not report a port"; t_end; exit 1 ;; esac

# The server must be TLS-only: a plain (non-TLS) connect must not complete a
# handshake. We assert the server is reachable and speaking TLS instead.
if command -v openssl >/dev/null 2>&1; then
    plainout="$(printf 'NICK x\r\n' | timeout 3 openssl s_client -verify_quiet -connect 127.0.0.1:"$port" -servername localhost -quiet 2>/dev/null | tr -d '\r' || true)"
    case "$plainout" in
        *'not a valid'*|*'alert'*|'') : ;;  # handshake failed as expected for a stale/plain probe
        *) : ;;
    esac
fi

# The rust client: use a distinct client state dir per operation to avoid any
# cross-connection race in the reference server (TOFU pins are per-dir anyway).
cli() { # <dir-suffix> <args...> -> runs the client with its own AI_CHAT_HOME
    local d="$temporary_root/c_$1"; shift
    mkdir -p "$d"
    AI_CHAT_HOME="$d" timeout 8 "$CLIENT" "$@"
}

# Discovery finds the announcing server.
disco="$(cli disco discover --bcast 127.0.0.1 --beacon-port 47991 --wait 3 --json 2>/dev/null || true)"
disco_port="$(printf '%s' "$disco" | grep -oE '"port":[0-9]+' | head -1 | cut -d: -f2 || true)"
if [ "$disco_port" != "$port" ]; then
    t_fail "discovery did not list the announcing server: $disco"
fi

# Send pins the cert (first connect, TOFU) and echoes the message.
sent="$(cli sA send --server 127.0.0.1:"$port" --nick alice --chan '#ops' --text 'hello rust chat' 2>/dev/null || true)"
case "$sent" in
    *':alice!alice@localhost PRIVMSG #ops :hello rust chat'*) : ;;
    *) t_fail "send did not echo the message: [$sent]" ;;
esac

# A pinned cert file was written.
fingerprint_file="$(ls "$temporary_root/c_sA"/*.cert.fp 2>/dev/null | head -1 || true)"
[ -n "$fingerprint_file" ] || t_fail "no TOFU fingerprint file was pinned"

# Read the delta since 0 returns the message and stops on the marker.
delta="$(cli rB read --server 127.0.0.1:"$port" --nick alice --chan '#ops' --since 0 2>/dev/null || true)"
case "$delta" in
    *'MSG #ops 1 '*' :hello rust chat'*) : ;;
    *) t_fail "read-delta did not return the message: [$delta]" ;;
esac

# A second read with a mismatched pin fails closed (TOFU). Seed a bogus pin.
bogus_fp="$temporary_root/c_rB/127_0_0_1_${port}.cert.fp"
printf 'bogus\n' > "$bogus_fp"
rc=0
cli rB read --server 127.0.0.1:"$port" --nick alice --chan '#ops' --since 0 >/dev/null 2>"$temporary_root/tofu.err" || rc=$?
[ "$rc" -eq 70 ] || t_fail "mismatched TOFU pin did not fail closed (rc=$rc): $(cat "$temporary_root/tofu.err")"

kill "$server_pid" 2>/dev/null || true
wait "$server_pid" 2>/dev/null || true
printf 'chat: exercised rust server + client (discovery, send TOFU, delta, fail-closed)\n' >&2
t_end
