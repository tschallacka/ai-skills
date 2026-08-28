#!/usr/bin/env bash
# MODE: DEV
# test-chat-watch-cursor.sh - B76: chat-watch.sh must advance its cursor.
#
# The helper built its chat-read argument list as (--since "$since") and then
# passed --since "$cursor" BEFORE it. chat-read's parser is last-wins, so the
# original since-id overrode the cursor on every poll and the watcher
# re-requested the window it started with, forever. Measured before the fix:
# each id emitted four times in twenty seconds, on the socket path and the
# local one alike, which makes the helper unusable as a session listener.
#
# The contract under test is the helper's own: each poll prints new messages
# only, and no message is missed. So this asserts BOTH halves — every id
# exactly once, and a message that arrives mid-run still delivered (the
# cursor must be the highest id printed, not one past it: every selection,
# local awk and every tier's FETCH, is strictly greater than since-id).
#
# The watch cadence starts at 5s, so the window has to span several polls;
# 13 seconds buys three. A re-emitting watcher shows a count of 3, not 1.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scripts="$root/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")/../../planning/tests" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C
fail() { t_fail "$*"; }

command -v timeout >/dev/null 2>&1 || {
    printf 'SKIP chat-watch-cursor: no timeout(1) - the watch assertions did not run\n' >&2
    t_end
}

temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/chat-watch.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

# <label> <emitted-ids-one-per-line> <expected-id-list-space-separated>
assert_each_once() {
    local label="$1" ids="$2" want="$3" got dupes
    got="$(printf '%s\n' "$ids" | sort -n | uniq | tr '\n' ' ' | sed 's/ *$//')"
    dupes="$(printf '%s\n' "$ids" | sort -n | uniq -c | awk '$1 > 1 { print $2 "x" $1 }' | tr '\n' ' ')"
    [ -z "$dupes" ] || fail "$label: ids re-emitted (id x count): $dupes"
    [ "$got" = "$want" ] || fail "$label: emitted ids [$got], expected [$want]"
}

home="$temporary_root/home"
"$scripts/chat-register.sh" '#watch' --home "$home" >/dev/null   # seeds id 1
"$scripts/chat-send.sh" '#watch' alpha -n w --home "$home" >/dev/null   # id 2
"$scripts/chat-send.sh" '#watch' beta  -n w --home "$home" >/dev/null   # id 3

# ---- local path: a quiet channel must not be re-read -----------------------
out="$(timeout 13 "$scripts/chat-watch.sh" '#watch' --since 0 --home "$home" \
        2>/dev/null | awk '$1 == "MSG" { print $3 }' || true)"
assert_each_once "local watch" "$out" "1 2 3"

# ---- socket path, and nothing skipped when a message arrives mid-run -------
runtime=""
for r in python3 node perl socat; do
    command -v "$r" >/dev/null 2>&1 && { runtime="$r"; break; }
done
if [ -z "$runtime" ]; then
    printf 'SKIP chat-watch-cursor: no server runtime present - the socket path was not exercised\n' >&2
else
    shome="$temporary_root/shome"
    extra=()
    [ "$runtime" = socat ] && extra=(--port 47935)
    if ! "$scripts/chat-server.sh" start --runtime "$runtime" "${extra[@]+"${extra[@]}"}" \
        --home "$shome" >"$temporary_root/start.log" 2>&1; then
        fail "server start failed on $runtime: $(cat "$temporary_root/start.log")"
    else
        port="$(cat "$shome/server.port")"
        "$scripts/chat-register.sh" '#watch' --home "$shome" >/dev/null   # id 1
        "$scripts/chat-send.sh" '#watch' alpha -n w --host 127.0.0.1 --port "$port" >/dev/null
        "$scripts/chat-send.sh" '#watch' beta  -n w --host 127.0.0.1 --port "$port" >/dev/null
        # id 4 lands after the first poll: it must be delivered, exactly once.
        ( sleep 6
          "$scripts/chat-send.sh" '#watch' gamma -n w --host 127.0.0.1 --port "$port" >/dev/null 2>&1
        ) &
        writer=$!
        out="$(timeout 16 "$scripts/chat-watch.sh" '#watch' --since 0 \
                --host 127.0.0.1 --port "$port" 2>/dev/null \
                | awk '$1 == "MSG" { print $3 }' || true)"
        wait "$writer" 2>/dev/null || true
        assert_each_once "socket watch ($runtime)" "$out" "1 2 3 4"
        "$scripts/chat-server.sh" stop --home "$shome" >/dev/null 2>&1 || true
    fi
fi

t_end
