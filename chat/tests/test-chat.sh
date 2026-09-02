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

SERVER="$repo/src/chat-server-rs/target/release/chat-server-rs"
CLIENT="$repo/src/chat-client-rs/target/release/chat-client-rs"

if ! command -v cargo >/dev/null 2>&1; then
    prebuilt_server="$(ls "$root"/bin/*/chat-server-rs 2>/dev/null | head -1 || true)"
    prebuilt_client="$(ls "$root"/bin/*/chat-client-rs 2>/dev/null | head -1 || true)"
    if [ -n "$prebuilt_server" ] && [ -n "$prebuilt_client" ]; then
        SERVER="$prebuilt_server"
        CLIENT="$prebuilt_client"
    else
        printf 'SKIP chat: no cargo and no prebuilt chat/bin binaries - rust assertions did not run\n' >&2
        t_end
        exit 0
    fi
else
    ( cd "$repo/src/chat-server-rs" && cargo build --release >/dev/null 2>&1 ) \
        || { t_fail "cargo build chat-server-rs failed"; }
    ( cd "$repo/src/chat-client-rs" && cargo build --release >/dev/null 2>&1 ) \
        || { t_fail "cargo build chat-client-rs failed"; }
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

# The mismatched-pin fail-closed path (TOFU).
bogus_fp="$temporary_root/c_rB/127_0_0_1_${port}.cert.fp"
printf 'bogus\n' > "$bogus_fp"
rc=0
cli rB read --server 127.0.0.1:"$port" --nick alice --chan '#ops' --since 0 >/dev/null 2>"$temporary_root/tofu.err" || rc=$?
[ "$rc" -eq 70 ] || t_fail "mismatched TOFU pin did not fail closed (rc=$rc): $(cat "$temporary_root/tofu.err")"

# An idle tail stays alive across a silence longer than the old 5s read
# timeout, and wakes when a message arrives on the channel it is watching.
wake_home="$temporary_root/wake"
mkdir -p "$wake_home"
AI_CHAT_HOME="$wake_home" "$CLIENT" tail --server 127.0.0.1:"$port" --nick wakee --chan '#wake' --insecure \
    </dev/null >>"$temporary_root/wake.log" 2>>"$temporary_root/wake.err" &
wake_pid=$!
sleep 2
if ! kill -0 "$wake_pid" 2>/dev/null; then
    t_fail "idle tail did not start: $(cat "$temporary_root/wake.err")"
fi
# Survive a silence past the old 5s read timeout.
sleep 6
if ! kill -0 "$wake_pid" 2>/dev/null; then
    t_fail "idle tail exited during a 6s silence: $(cat "$temporary_root/wake.err")"
fi
# Send a wake message; the tail's poll must pick it up and print it.
wake_msg="wake wakee now"
cli wS send --server 127.0.0.1:"$port" --nick waker --chan '#wake' --text "$wake_msg" >/dev/null 2>"$temporary_root/wake-send.err" \
    || t_fail "wake send failed: $(cat "$temporary_root/wake-send.err")"
sleep 8 # allow the tail's poll (5s interval) to catch it
grep -q "$wake_msg" "$temporary_root/wake.log" \
    || t_fail "idle tail did not wake on the message: [$(cat "$temporary_root/wake.log")]"
if ! kill -0 "$wake_pid" 2>/dev/null; then
    t_fail "tail exited after waking"
fi
kill "$wake_pid" 2>/dev/null || true
wait "$wake_pid" 2>/dev/null || true

# A session remembers server+nick and per-channel cursors, so send/read can
# omit --server/--nick/--since; a malformed session.json recovers with a warning.
session_home="$temporary_root/session"
mkdir -p "$session_home"
cli sess session set --server 127.0.0.1:"$port" --nick sessioner >/dev/null 2>&1
shown="$(cli sess session show 2>/dev/null || true)"
case "$shown" in
    *"server=127.0.0.1:$port"*"nick=sessioner"*) : ;;
    *) t_fail "session set/show did not persist server+nick: [$shown]" ;;
esac
# send WITHOUT --server/--nick uses the session, on a fresh channel so the
# cursor count is deterministic.
sent2="$(cli sess send --chan '#sess' --text 'session message' --insecure 2>/dev/null || true)"
case "$sent2" in
    *':sessioner!sessioner@localhost PRIVMSG #sess :session message'*) : ;;
    *) t_fail "session-backed send failed: [$sent2]" ;;
esac
# send advances the channel cursor; a later no-arg send works too.
cli sess send --chan '#sess' --text 'second session' --insecure >/dev/null 2>&1 || true
cursor="$(cli sess session show 2>/dev/null | grep 'cursor #sess' | awk '{print $NF}' || true)"
[ "$cursor" = "2" ] || t_fail "session cursor did not advance to 2: [$cursor]"
# A malformed session file must recover (warning + empty session), not crash.
# Sessions are per-owner under sessions/<owner>.json, so corrupt whatever file
# the client actually wrote rather than a fixed name — writing session.json
# would corrupt nothing and let this assertion pass without testing anything.
session_file="$(ls "$temporary_root/c_sess/sessions/"*.json 2>/dev/null | head -1 || true)"
[ -n "$session_file" ] \
    || t_fail "no per-owner session file was written under $temporary_root/c_sess/sessions/"
printf '{ broken json !!!' > "$session_file"
recovered="$(cli sess session show 2>/dev/null || true)"
case "$recovered" in
    *'server='*) : ;;
    *) t_fail "malformed session did not recover: [$recovered]" ;;
esac
# --no-session ignores the saved server/nick (send without them fails).
rc=0
cli sess send --chan '#x' --text 'x' --insecure --no-session >/dev/null 2>"$temporary_root/nosession.err" || rc=$?
[ "$rc" -eq 64 ] || t_fail "--no-session send without server/nick exited $rc (want 64)"

# join seeds the cursor to the channel's CURRENT end (no history dump); read
# after join resumes from the cursor; leave drops the cursor. Mentions: a tail
# --mentions --mention-exit catches a concurrent @<nick> and exits.
# (Re-set the session: the malformed-JSON recovery above reset it.)
cli sess session set --server 127.0.0.1:"$port" --nick sessioner >/dev/null 2>&1 || true
cli sess send --chan '#old' --text 'ancient' --insecure >/dev/null 2>&1 || true
cli sess send --chan '#old' --text 'elder' --insecure >/dev/null 2>&1 || true
j="$(cli sess join --chan '#old' --insecure 2>&1)"
case "$j" in
    *'resuming after id 2'*) : ;;
    *) t_fail "join did not seed to current end: [$j]" ;;
esac
# read after join without --since must not dump the old history.
after="$(cli sess read --chan '#old' --insecure 2>&1)"
[ -z "$after" ] || t_fail "read after join dumped history: [$after]"
# explicit --since 0 still reads everything.
hist="$(cli sess read --chan '#old' --since 0 --insecure 2>&1)"
case "$hist" in
    *'ancient'*'elder'*) : ;;
    *) t_fail "read --since 0 did not return full history: [$hist]" ;;
esac
# leave drops the cursor from the session.
cli sess leave --chan '#old' --insecure >/dev/null 2>&1 || true
cleft="$(cli sess session show 2>/dev/null | grep 'cursor #old' || true)"
[ -z "$cleft" ] || t_fail "leave did not drop the #old cursor: [$cleft]"
# mention-notify: a tail --mentions --mention-exit exits when a concurrent
# send mentions @<session nick> (the sender auto-suffixes on nick-in-use).
mhome="$temporary_root/ment"
mkdir -p "$mhome"
AI_CHAT_HOME="$mhome" "$CLIENT" session set --server 127.0.0.1:"$port" --nick mwatcher >/dev/null 2>&1 || true
AI_CHAT_HOME="$mhome" "$CLIENT" join --chan '#ment' --insecure >/dev/null 2>&1 || true
AI_CHAT_HOME="$mhome" "$CLIENT" tail --chan '#ment' --mentions --mention-exit --insecure \
    >>"$temporary_root/ment.log" 2>>"$temporary_root/ment.err" &
ment_pid=$!
sleep 5
AI_CHAT_HOME="$mhome" "$CLIENT" send --chan '#ment' --text 'ping @mwatcher now' --insecure \
    >/dev/null 2>>"$temporary_root/ment-send.err" || t_fail "mention send failed: $(cat "$temporary_root/ment-send.err")"
for i in $(seq 1 6); do
    kill -0 "$ment_pid" 2>/dev/null || break
    sleep 2
done
if kill -0 "$ment_pid" 2>/dev/null; then
    t_fail "tail --mentions --mention-exit did not exit on a mention"
fi
grep -q '!! MENTION !!' "$temporary_root/ment.log" \
    || t_fail "mention was not surfaced: [$(cat "$temporary_root/ment.log")]"
wait "$ment_pid" 2>/dev/null || true

# Two agents on ONE machine must each keep their own identity. A single shared
# session file used to mean the second agent's nick replaced the first's, so
# the first then read and posted as the second - and only one of them could
# stay connected at all.
two_home="$temporary_root/two"
mkdir -p "$two_home"
CHAT_SESSION_ID=agent-one AI_CHAT_HOME="$two_home" "$CLIENT" session set \
    --server 127.0.0.1:"$port" --nick one >/dev/null 2>&1 || true
CHAT_SESSION_ID=agent-two AI_CHAT_HOME="$two_home" "$CLIENT" session set \
    --server 127.0.0.1:"$port" --nick two >/dev/null 2>&1 || true
one_nick="$(CHAT_SESSION_ID=agent-one AI_CHAT_HOME="$two_home" "$CLIENT" session show 2>/dev/null \
    | awk -F= '/^nick=/{print $2}')"
two_nick="$(CHAT_SESSION_ID=agent-two AI_CHAT_HOME="$two_home" "$CLIENT" session show 2>/dev/null \
    | awk -F= '/^nick=/{print $2}')"
[ "$one_nick" = one ] \
    || t_fail "agent-one's session nick was clobbered: [$one_nick] (want one)"
[ "$two_nick" = two ] \
    || t_fail "agent-two's session nick was clobbered: [$two_nick] (want two)"

# Both must be able to hold a nick at the same time: a disconnect has to
# release its nick registration, or the second agent gets ERR_NICKNAMEINUSE
# against a connection that no longer exists.
CHAT_SESSION_ID=agent-one AI_CHAT_HOME="$two_home" "$CLIENT" send --chan '#two' \
    --text 'from one' --insecure >/dev/null 2>&1 \
    || t_fail "agent-one could not send"
CHAT_SESSION_ID=agent-two AI_CHAT_HOME="$two_home" "$CLIENT" send --chan '#two' \
    --text 'from two' --insecure >/dev/null 2>&1 \
    || t_fail "agent-two could not send (nick released on disconnect?)"
# And the same nick must be able to come back, repeatedly.
for _ in 1 2 3; do
    CHAT_SESSION_ID=agent-one AI_CHAT_HOME="$two_home" "$CLIENT" send --chan '#two' \
        --text 'again' --insecure >/dev/null 2>&1 \
        || t_fail "reconnecting as the same nick was refused"
done

# Taking over another agent's session must WARN, not happen silently.
takeover_err="$temporary_root/takeover.err"
CHAT_SESSION_ID=agent-one AI_CHAT_HOME="$two_home" "$CLIENT" send --chan '#two' \
    --text 'takeover' --nick someone-else --insecure >/dev/null 2>"$takeover_err" || true
grep -q "already belongs to nick" "$takeover_err" \
    || t_fail "a session takeover was silent: [$(cat "$takeover_err")]"

kill "$server_pid" 2>/dev/null || true
wait "$server_pid" 2>/dev/null || true

# --- with the server STOPPED ------------------------------------------------
# read --local must still work: the channel log is the storage format, so a
# local read returns the rows a FETCH would. Without this there is no supported
# way to see a channel when no server is running, and the only recourse is
# opening the log by hand.
local_out="$(AI_CHAT_HOME="$home" "$CLIENT" read --local --chan '#sess' --since 0 --no-session 2>/dev/null || true)"
case "$local_out" in
    *'MSG #sess'*'session message'*) : ;;
    *) t_fail "read --local returned nothing with the server stopped: [$local_out]" ;;
esac
# --since must bound a local read the same way it bounds a FETCH.
bounded="$(AI_CHAT_HOME="$home" "$CLIENT" read --local --chan '#sess' --since 1 --no-session 2>/dev/null | grep -c '^MSG ' || true)"
unbounded="$(AI_CHAT_HOME="$home" "$CLIENT" read --local --chan '#sess' --since 0 --no-session 2>/dev/null | grep -c '^MSG ' || true)"
[ "$bounded" -lt "$unbounded" ] \
    || t_fail "read --local ignored --since (bounded=$bounded unbounded=$unbounded)"
# An unknown channel is an actionable error, not a crash or silence.
rc=0
AI_CHAT_HOME="$home" "$CLIENT" read --local --chan '#nosuchchannel' --no-session \
    >/dev/null 2>"$temporary_root/nochan.err" || rc=$?
[ "$rc" -eq 66 ] \
    || t_fail "read --local on a missing channel exited $rc (want 66)"

# The server must answer --help WITHOUT binding: it used to parse argv[1] as a
# port, so --help fell through and stood up a real server on the shared port.
help_home="$temporary_root/helphome"
mkdir -p "$help_home"
AI_CHAT_HOME="$help_home" "$SERVER" --help >"$temporary_root/help.out" 2>&1 &
help_pid=$!
sleep 2
if kill -0 "$help_pid" 2>/dev/null; then
    kill -9 "$help_pid" 2>/dev/null || true
    t_fail "server --help did not exit; it is still running (it bound a socket)"
else
    wait "$help_pid" 2>/dev/null
    help_rc=$?
    [ "$help_rc" -eq 0 ] || t_fail "server --help exited $help_rc (want 0)"
fi
[ -s "$temporary_root/help.out" ] || t_fail "server --help printed nothing"
[ -f "$help_home/server.port" ] \
    && t_fail "server --help wrote server.port; it must not touch the home"
# A typo must be refused, not silently read as "no port given".
rc=0
AI_CHAT_HOME="$help_home" "$SERVER" --porrt 1234 >/dev/null 2>&1 || rc=$?
[ "$rc" -eq 64 ] || t_fail "server with a bad argument exited $rc (want 64)"

# The client's --state must actually be honoured: it was documented but had no
# case in the parser, so it was silently discarded and every call fell back to
# $AI_CHAT_HOME - two agents told to keep separate state shared one directory.
state_dir="$temporary_root/explicit-state"
AI_CHAT_HOME="$temporary_root/ignored" "$CLIENT" session set --state "$state_dir" \
    --server 127.0.0.1:1 --nick stateful >/dev/null 2>&1 || true
[ -d "$state_dir/sessions" ] \
    || t_fail "--state was ignored: nothing was written under $state_dir"
[ -d "$temporary_root/ignored/sessions" ] \
    && t_fail "--state was ignored: the session went to AI_CHAT_HOME instead"
# An unknown option must be an error, not silently swallowed.
rc=0
"$CLIENT" session show --state "$state_dir" --nosuchflag >/dev/null 2>&1 || rc=$?
[ "$rc" -eq 64 ] || t_fail "an unknown client option exited $rc (want 64)"
# --help is a request, not a mistake: stdout, exit 0.
"$CLIENT" --help >"$temporary_root/clienthelp.out" 2>/dev/null \
    || t_fail "client --help did not exit 0"
[ -s "$temporary_root/clienthelp.out" ] \
    || t_fail "client --help printed nothing on stdout"

printf 'chat: exercised rust server + client (discovery, send TOFU, delta, fail-closed, idle tail wakes, session, join/leave, mentions, per-agent sessions, nick release, local reads, --help, --state)\n' >&2
t_end
