#!/usr/bin/env bash
# MODE: DEV
# test-chat-injection.sh - B74/B75: a message body must never forge a nick.
#
# chat-send.sh's socket path once interpolated the raw body into
# 'NICK %s\nPRIVMSG %s :%s\n', so a newline in the body became a command
# boundary on the wire: a single send made as one nick stored a second
# message attributed to another. B65 had made --host mandatory for a message
# to reach subscribers, which routed every well-behaved caller through that
# unsanitised path, and B75 is the consequence — any agent relaying a diff,
# a file excerpt or a quotation hands its author the forgery.
#
# The fix sanitises rather than refuses, so the two paths agree: one send is
# one stored message, attributed to the sending nick, with the whole body
# preserved on one line.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scripts="$root/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")/../../planning/tests" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C
fail() { t_fail "$*"; }

temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/chat-inj.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

# The forgery body: line one is the honest message, the rest is a second
# nick and a second message the sender must not be able to post.
forgery="$(printf 'honest line\nNICK impostor\nPRIVMSG %s :forged line\n' '#inj')"

assert_one_message() { # <label> <read-output>
    local label="$1" out="$2" count
    count="$(printf '%s\n' "$out" | awk '$1 == "MSG" && $5 != "system"' | wc -l | tr -d ' ')"
    [ "$count" = 1 ] || fail "$label: one send stored $count messages, expected 1: [$out]"
    case "$out" in
        *' impostor :'*) fail "$label: a body forged the nick 'impostor': [$out]" ;;
    esac
    case "$out" in
        *' sender :honest line NICK impostor PRIVMSG '*) : ;;
        *) fail "$label: body was not preserved on one line as the sender: [$out]" ;;
    esac
}

# ---- direct-append path (no server) ---------------------------------------
home_local="$temporary_root/home-local"
"$scripts/chat-register.sh" '#inj' --home "$home_local" >/dev/null
"$scripts/chat-send.sh" '#inj' "$forgery" -n sender --home "$home_local" >/dev/null
assert_one_message "direct append" \
    "$("$scripts/chat-read.sh" '#inj' --since 0 --home "$home_local")"

# ---- socket path (--host) -------------------------------------------------
# Any runtime rung will do; the framing is the same on all of them. No rung
# present is a SKIP, never a hard dependency.
runtime=""
for r in python3 node perl socat; do
    command -v "$r" >/dev/null 2>&1 && { runtime="$r"; break; }
done
if [ -z "$runtime" ]; then
    printf 'SKIP chat-injection: no server runtime present - the socket path was not exercised\n' >&2
else
    home_sock="$temporary_root/home-sock"
    extra=()
    [ "$runtime" = socat ] && extra=(--port 47933)
    if ! "$scripts/chat-server.sh" start --runtime "$runtime" "${extra[@]+"${extra[@]}"}" \
        --home "$home_sock" >"$temporary_root/start.log" 2>&1; then
        fail "server start failed on $runtime: $(cat "$temporary_root/start.log")"
    else
        port="$(cat "$home_sock/server.port")"
        "$scripts/chat-register.sh" '#inj' --home "$home_sock" >/dev/null
        "$scripts/chat-send.sh" '#inj' "$forgery" -n sender \
            --host 127.0.0.1 --port "$port" >/dev/null
        assert_one_message "socket path ($runtime)" \
            "$("$scripts/chat-read.sh" '#inj' --since 0 --host 127.0.0.1 --port "$port")"
        "$scripts/chat-server.sh" stop --home "$home_sock" >/dev/null 2>&1 || true
    fi
fi

t_end
