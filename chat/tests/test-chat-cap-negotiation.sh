#!/usr/bin/env bash
# MODE: DEV
# test-chat-cap-negotiation.sh - B157/T131: real IRCv3 CAP negotiation and the
# message-tags msgid it carries on a broadcast PRIVMSG.
#
# chat-client-rs does not yet speak CAP (that is T135), so every connection
# here is raw IRC over TLS via `openssl s_client`, exactly like the broadcast
# watcher in test-chat-broadcast-stall.sh: its stdin is a fifo held open by a
# background feeder, because s_client exits the moment stdin closes. This is
# also why the T134 (message-tags on broadcast) assertions do not need T135 to
# exist yet: which line a RECIPIENT gets depends only on what that recipient
# negotiated, never on what the sender did.
#
# A missing cargo/openssl/mkfifo is a loud SKIP, not a failure.

set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo="$(cd "$root/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$repo/planning/tests" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

if ! command -v openssl >/dev/null 2>&1 || ! command -v mkfifo >/dev/null 2>&1; then
    t_skip 'chat cap negotiation: no openssl or mkfifo - raw protocol probing did not run'
fi
if ! command -v timeout >/dev/null 2>&1; then
    t_skip 'chat cap negotiation: no timeout(1) - the wait-bounded assertions need it'
fi

SERVER="$repo/target/release/chat-server-rs"
if ! command -v cargo >/dev/null 2>&1; then
    prebuilt_server="$(find "$root/bin" -type f -name chat-server-rs 2>/dev/null | head -1)"
    if [ -n "$prebuilt_server" ]; then
        SERVER="$prebuilt_server"
    else
        t_skip 'chat cap negotiation: no cargo and no prebuilt chat/bin/*/chat-server-rs'
    fi
else
    ( cd "$repo/src/chat-server-rs" && cargo build --release >/dev/null 2>&1 ) \
        || t_fail "cargo build chat-server-rs failed"
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/chat-cap.XXXXXX")"
home="$work/home"
mkdir -p "$home"
server_pid=""
cleanup() {
    [ -n "$server_pid" ] && { kill "$server_pid" 2>/dev/null || :; }
    rm -rf "$work"
    return 0
}
trap cleanup EXIT

# A beacon port nothing else uses (B274: never announce on the default port
# from a throwaway test server).
AI_CHAT_HOME="$home" CHAT_BEACON_PORT=47993 \
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

# Opens a raw TLS connection whose stdin is a fifo: `$1` names it (used for the
# fifo/output file paths), `$2` is the full command burst to send immediately,
# `$3` (optional) is how long to keep the connection open afterwards (seconds;
# default 6, long enough for every assertion below to poll its output).
open_raw() { # <name> <initial-burst> [hold-seconds]
    local name="$1" burst="$2" hold="${3:-6}"
    mkfifo "$work/$name.in"
    ( printf '%b' "$burst"; sleep "$hold" ) >"$work/$name.in" &
    eval "${name}_feeder=\$!"
    timeout "$((hold + 3))" openssl s_client -quiet -verify_quiet \
        -connect 127.0.0.1:"$port" -servername localhost \
        <"$work/$name.in" >"$work/$name.out" 2>/dev/null &
    eval "${name}_pid=\$!"
}

# Sends more lines into an already-open raw connection's fifo. Only safe
# while the connection's `open_raw` hold window has not yet elapsed.
feed_raw() { # <name> <more-lines>
    printf '%b' "$2" >"$work/$1.in"
}

out_of() { cat "$work/$1.out" 2>/dev/null || true; } # <name>

wait_for() { # <name> <needle> <seconds> -> 0 once the output contains needle
    local name="$1" needle="$2" seconds="$3" i=0
    while [ "$i" -lt $((seconds * 2)) ]; do
        case "$(out_of "$name")" in
            *"$needle"*) return 0 ;;
        esac
        sleep 0.5
        i=$((i + 1))
    done
    return 1
}

# ── CAP LS lists exactly the registered capabilities ─────────────────────────
open_raw ls_only 'CAP LS\r\n'
if wait_for ls_only 'CAP * LS' 5; then
    case "$(out_of ls_only)" in
        *'CAP * LS :message-tags'*) ;;
        *) t_fail "CAP LS did not list exactly message-tags: $(out_of ls_only)" ;;
    esac
else
    t_fail "CAP LS produced no reply at all"
fi
kill "${ls_only_pid:-}" "${ls_only_feeder:-}" 2>/dev/null || :

# ── REQ of a supported capability ACKs ───────────────────────────────────────
open_raw req_ok 'CAP REQ :message-tags\r\n'
if wait_for req_ok 'CAP *' 5; then
    case "$(out_of req_ok)" in
        *'CAP * ACK :message-tags'*) ;;
        *) t_fail "CAP REQ message-tags did not ACK: $(out_of req_ok)" ;;
    esac
else
    t_fail "CAP REQ message-tags produced no reply"
fi
kill "${req_ok_pid:-}" "${req_ok_feeder:-}" 2>/dev/null || :

# ── REQ of an unsupported capability NAKs ────────────────────────────────────
open_raw req_bad 'CAP REQ :no-such-capability\r\n'
if wait_for req_bad 'CAP *' 5; then
    case "$(out_of req_bad)" in
        *'CAP * NAK :no-such-capability'*) ;;
        *) t_fail "CAP REQ of an unsupported capability did not NAK: $(out_of req_bad)" ;;
    esac
else
    t_fail "CAP REQ no-such-capability produced no reply"
fi
kill "${req_bad_pid:-}" "${req_bad_feeder:-}" 2>/dev/null || :

# ── a mixed REQ (one supported, one not) NAKs the whole request ─────────────
open_raw req_mixed 'CAP REQ :message-tags no-such-capability\r\n'
if wait_for req_mixed 'CAP *' 5; then
    case "$(out_of req_mixed)" in
        *'CAP * NAK :message-tags no-such-capability'*) ;;
        *) t_fail "a mixed CAP REQ did not NAK the whole set: $(out_of req_mixed)" ;;
    esac
else
    t_fail "the mixed CAP REQ produced no reply"
fi
kill "${req_mixed_pid:-}" "${req_mixed_feeder:-}" 2>/dev/null || :

# ── registration is held across CAP LS ... CAP END, and completes on END ────
open_raw held \
    'CAP LS\r\nNICK capheld\r\nUSER capheld 0 * :capheld\r\n' 8
sleep 2
case "$(out_of held)" in
    *' 001 '*) t_fail "registration completed before CAP END: $(out_of held)" ;;
esac
feed_raw held 'CAP END\r\n'
if ! wait_for held ' 001 ' 5; then
    t_fail "registration never completed after CAP END: $(out_of held)"
fi
kill "${held_pid:-}" "${held_feeder:-}" 2>/dev/null || :

# ── a plain NICK/USER client with no CAP registers exactly as before ────────
# (regression pin: T133 must not have added a hold that catches a client which
# never mentions CAP at all.)
open_raw plain_registers 'NICK plainreg\r\nUSER plainreg 0 * :plainreg\r\n'
if ! wait_for plain_registers ' 001 ' 5; then
    t_fail "a plain NICK/USER client (no CAP at all) never registered: $(out_of plain_registers)"
fi
kill "${plain_registers_pid:-}" "${plain_registers_feeder:-}" 2>/dev/null || :

# ── two connections negotiate independently ──────────────────────────────────
open_raw indep_a 'CAP LS\r\nCAP REQ :message-tags\r\nNICK indepa\r\nUSER indepa 0 * :a\r\nCAP END\r\n'
open_raw indep_b 'NICK indepb\r\nUSER indepb 0 * :b\r\n'
wait_for indep_a ' 001 ' 5 || t_fail "connection A never registered: $(out_of indep_a)"
wait_for indep_b ' 001 ' 5 || t_fail "connection B never registered: $(out_of indep_b)"
case "$(out_of indep_a)" in
    *'CAP * ACK :message-tags'*) ;;
    *) t_fail "connection A's own negotiation did not ACK: $(out_of indep_a)" ;;
esac
case "$(out_of indep_b)" in
    *'CAP *'*) t_fail "connection B, which never sent CAP, got a CAP reply anyway: $(out_of indep_b)" ;;
esac
kill "${indep_a_pid:-}" "${indep_a_feeder:-}" "${indep_b_pid:-}" "${indep_b_feeder:-}" 2>/dev/null || :

# ── T134: a negotiated peer gets the msgid tag, a plain peer in the SAME
#    channel does not, from the SAME broadcast ────────────────────────────────
chan='#capnego'
open_raw tagged_peer \
    "CAP LS\r\nCAP REQ :message-tags\r\nNICK tagpeer\r\nUSER tagpeer 0 * :t\r\nCAP END\r\nJOIN $chan\r\n" 8
open_raw plain_peer \
    "NICK plainpeer\r\nUSER plainpeer 0 * :p\r\nJOIN $chan\r\n" 8
wait_for tagged_peer 'End of /NAMES list' 5 \
    || t_fail "the tagged peer never completed its JOIN: $(out_of tagged_peer)"
wait_for plain_peer 'End of /NAMES list' 5 \
    || t_fail "the plain peer never completed its JOIN: $(out_of plain_peer)"

open_raw sender \
    "NICK capsender\r\nUSER capsender 0 * :s\r\nJOIN $chan\r\nPRIVMSG $chan :tagged-vs-plain\r\n" 6
wait_for sender 'End of /NAMES list' 5 || t_fail "the sender never joined: $(out_of sender)"

if wait_for tagged_peer 'tagged-vs-plain' 5; then
    case "$(out_of tagged_peer)" in
        *"@msgid="*"PRIVMSG $chan :tagged-vs-plain"*) ;;
        *) t_fail "the message-tags-negotiated peer did not get a msgid tag: $(out_of tagged_peer)" ;;
    esac
else
    t_fail "the tagged peer never received the broadcast at all: $(out_of tagged_peer)"
fi
if wait_for plain_peer 'tagged-vs-plain' 5; then
    case "$(out_of plain_peer)" in
        *'@msgid='*) t_fail "a peer that never negotiated message-tags got a tag anyway: $(out_of plain_peer)" ;;
        *":capsender!"*"PRIVMSG $chan :tagged-vs-plain"*) ;;
        *) t_fail "the plain peer's line did not match the expected untagged shape: $(out_of plain_peer)" ;;
    esac
else
    t_fail "the plain peer never received the broadcast at all: $(out_of plain_peer)"
fi
kill "${tagged_peer_pid:-}" "${tagged_peer_feeder:-}" \
    "${plain_peer_pid:-}" "${plain_peer_feeder:-}" \
    "${sender_pid:-}" "${sender_feeder:-}" 2>/dev/null || :

# ── the server process itself must still be alive: nothing above may have
#    been bought with a panicked handler thread ──────────────────────────────
srv_state="$(ps -o state= -p "$server_pid" 2>/dev/null | tr -d ' ')"
case "$srv_state" in
    ''|Z*) t_fail "the server process died during the run (state '${srv_state:-gone}')" ;;
esac
case "$(cat "$work/server.err" 2>/dev/null)" in
    *panic*) t_fail "the server logged a panic: $(cat "$work/server.err")" ;;
esac

[ "$(t_failures)" -eq 0 ] || exit 1
printf '%s\n' 'test-chat-cap-negotiation: PASS'
