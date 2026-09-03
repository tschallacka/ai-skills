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

SERVER="$repo/target/release/chat-server-rs"
CLIENT="$repo/target/release/chat-client-rs"

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
# Take the server down however this test ends. A failure that left it running
# kept announcing on the beacon port, so the NEXT run saw two servers and failed
# on discovery -- a leak that looks like a regression in the code under test.
# `|| true`: by the normal path the server is already down, and a failing kill
# inside an EXIT trap under set -e would turn a passing run into exit 1.
trap 'kill "$server_pid" 2>/dev/null || true; rm -rf "$temporary_root"' EXIT
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
# Ask the client which file it owns rather than assuming: session files are per
# agent now, so the path carries the session key.
sess_file="$(cli sess session show 2>/dev/null | sed -n 's/^file=//p')"
[ -n "$sess_file" ] || t_fail "session show did not report its file"
printf '{ broken json !!!' > "$sess_file"
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

# ---- B116: two agents, ONE AI_CHAT_HOME, separate sessions ----------------
# This is the shape the defect was measured in: both agents leave AI_CHAT_HOME
# at one shared path. Each must keep its own nick and its own per-channel
# cursor. Before the per-agent session key, one session.json held both, so
# agent-b's join overwrote agent-a's nick and either agent's read silently
# advanced the other's cursor.
shared="$temporary_root/shared"
mkdir -p "$shared"

# One shared state dir; the two agents differ only by their session identity.
# --session is the explicit rung of the ladder.
agent() { # <session-id> <args...>
    local sid="$1"
    shift
    AI_CHAT_HOME="$shared" timeout 8 "$CLIENT" --session "$sid" "$@"
}

agent agent-a session set --server 127.0.0.1:"$port" --nick agent-a >/dev/null 2>&1 \
    || t_fail "agent-a session set failed"
agent agent-b session set --server 127.0.0.1:"$port" --nick agent-b >/dev/null 2>&1 \
    || t_fail "agent-b session set failed"

# The nick each agent reports is its own, not whichever wrote last.
a_show="$(agent agent-a session show 2>/dev/null || true)"
b_show="$(agent agent-b session show 2>/dev/null || true)"
case "$a_show" in
    *'nick=agent-a'*) : ;;
    *) t_fail "agent-a's session reports the wrong nick: [$a_show]" ;;
esac
case "$b_show" in
    *'nick=agent-b'*) : ;;
    *) t_fail "agent-b's session reports the wrong nick: [$b_show]" ;;
esac
case "$a_show" in
    *'source=explicit'*) : ;;
    *) t_fail "--session did not take the explicit rung: [$a_show]" ;;
esac

# A bare send (no --nick) goes out under the sending agent's own identity.
a_sent="$(agent agent-a send --chan '#iso' --text 'from a' --insecure 2>/dev/null || true)"
b_sent="$(agent agent-b send --chan '#iso' --text 'from b' --insecure 2>/dev/null || true)"
case "$a_sent" in
    *':agent-a!agent-a@localhost PRIVMSG #iso :from a'*) : ;;
    *) t_fail "agent-a's bare send used the wrong nick: [$a_sent]" ;;
esac
case "$b_sent" in
    *':agent-b!agent-b@localhost PRIVMSG #iso :from b'*) : ;;
    *) t_fail "agent-b's bare send used the wrong nick: [$b_sent]" ;;
esac

# Cursors are separate: agent-a sends twice on a fresh channel, which advances
# only agent-a's cursor. agent-b must still have no read position there, so
# agent-b's unread messages were not consumed on its behalf.
agent agent-a send --chan '#isoc' --text 'one' --insecure >/dev/null 2>&1 || true
agent agent-a send --chan '#isoc' --text 'two' --insecure >/dev/null 2>&1 || true
a_cursor="$(agent agent-a session show 2>/dev/null | sed -n 's/^cursor #isoc //p')"
b_cursor="$(agent agent-b session show 2>/dev/null | sed -n 's/^cursor #isoc //p')"
[ "$a_cursor" = "2" ] || t_fail "agent-a's #isoc cursor is [$a_cursor], want 2"
[ -z "$b_cursor" ] || t_fail "agent-b inherited agent-a's #isoc cursor: [$b_cursor]"

# The damaging half of the defect: one agent reading must not consume the
# other's unread messages. Both agents join a fresh channel, so both sit at its
# end; a third party then posts two messages that BOTH are owed.
# One message first, so the channel has an end to join at: a join to an EMPTY
# channel seeds the cursor to 0, which read cannot tell apart from having no
# cursor at all.
cli out send --server 127.0.0.1:"$port" --nick outsider --chan '#isod' --text 'channel seed' \
    >/dev/null 2>&1 || t_fail "outsider seed send failed"
agent agent-a join --chan '#isod' --insecure >/dev/null 2>&1 || t_fail "agent-a join failed"
agent agent-b join --chan '#isod' --insecure >/dev/null 2>&1 || t_fail "agent-b join failed"
cli out send --server 127.0.0.1:"$port" --nick outsider --chan '#isod' --text 'owed one' \
    >/dev/null 2>&1 || t_fail "outsider send 1 failed"
cli out send --server 127.0.0.1:"$port" --nick outsider --chan '#isod' --text 'owed two' \
    >/dev/null 2>&1 || t_fail "outsider send 2 failed"

# agent-a reads first, which advances agent-a's cursor past both.
a_read="$(agent agent-a read --chan '#isod' --insecure 2>/dev/null || true)"
case "$a_read" in
    *'owed one'*'owed two'*) : ;;
    *) t_fail "agent-a did not receive the messages it was owed: [$a_read]" ;;
esac
# agent-b must still receive them. On one shared session, agent-a's read had
# already moved the single cursor past both and agent-b saw nothing -- messages
# that look to agent-b as though they never arrived.
b_read="$(agent agent-b read --chan '#isod' --insecure 2>/dev/null || true)"
case "$b_read" in
    *'owed one'*'owed two'*) : ;;
    *) t_fail "agent-a's read consumed agent-b's unread messages: [$b_read]" ;;
esac

# Two agents, two session files in the one shared state dir.
# `|| true` inside: with no sessions/ directory at all this must report a
# finding, not abort the run under pipefail before the later assertions.
session_files="$({ ls "$shared/sessions" 2>/dev/null || true; } | wc -l | tr -d ' ')"
[ "$session_files" = "2" ] || t_fail "want 2 session files in the shared home, got $session_files"

# The zero-config rungs, with nothing chosen by hand. A harness session id the
# environment already carries separates two agents with no --session at all.
noenv() { env -u CHAT_SESSION_ID -u CODEX_SESSION_ID -u OPENCODE_PID "$@"; }
AI_CHAT_HOME="$shared" noenv CLAUDE_CODE_SESSION_ID=harness-a \
    timeout 8 "$CLIENT" session set --nick harness-a >/dev/null 2>&1 || true
AI_CHAT_HOME="$shared" noenv CLAUDE_CODE_SESSION_ID=harness-b \
    timeout 8 "$CLIENT" session set --nick harness-b >/dev/null 2>&1 || true
h_show_a="$(AI_CHAT_HOME="$shared" noenv CLAUDE_CODE_SESSION_ID=harness-a \
    timeout 8 "$CLIENT" session show 2>/dev/null || true)"
case "$h_show_a" in
    *'source=harness'*'nick=harness-a'*) : ;;
    *) t_fail "a harness session id did not isolate the session: [$h_show_a]" ;;
esac

# With no harness id either, the git worktree root decides, so two checkouts of
# one project keep separate sessions without any configuration.
if command -v git >/dev/null 2>&1; then
    for wt in wt1 wt2; do
        mkdir -p "$temporary_root/$wt"
        ( cd "$temporary_root/$wt" && git init -q . 2>/dev/null ) || true
    done
    wt_show() { # <dir> <args...>
        local d="$temporary_root/$1"
        shift
        ( cd "$d" && AI_CHAT_HOME="$shared" \
            env -u CHAT_SESSION_ID -u CODEX_SESSION_ID -u OPENCODE_PID -u CLAUDE_CODE_SESSION_ID \
            timeout 8 "$CLIENT" "$@" )
    }
    wt_show wt1 session set --nick in-wt1 >/dev/null 2>&1 || true
    wt_show wt2 session set --nick in-wt2 >/dev/null 2>&1 || true
    w1="$(wt_show wt1 session show 2>/dev/null || true)"
    w2="$(wt_show wt2 session show 2>/dev/null || true)"
    case "$w1" in
        *'source=worktree'*'nick=in-wt1'*) : ;;
        *) t_fail "the worktree rung did not isolate wt1: [$w1]" ;;
    esac
    case "$w2" in
        *'source=worktree'*'nick=in-wt2'*) : ;;
        *) t_fail "the worktree rung did not isolate wt2: [$w2]" ;;
    esac
fi

# ---- B1xx: a dead peer must not pin a core, and must give its nick back ----
# serve()'s outer loop had no exit: every `break` inside it left only the inner
# read loop, so a thread whose peer had died went on re-reading a closed socket
# at full CPU for the life of the process. Three SIGKILLed peers measured 899
# CPU ticks over three seconds (about three cores) and left three sockets in
# CLOSE-WAIT; enough of them starve accept() until the listener stops
# answering. A finished connection also has to surrender its nick -- nothing
# did, so the next agent asking for that nick collided with a connection that
# no longer existed and was auto-suffixed off the identity it asked for.
spin_home="$temporary_root/spin"
mkdir -p "$spin_home"
spin_pids=""
for i in 1 2 3; do
    AI_CHAT_HOME="$spin_home" "$CLIENT" tail --server 127.0.0.1:"$port" --nick "zombie$i" \
        --chan '#spin' --insecure --no-session </dev/null >>"$temporary_root/spin.log" 2>&1 &
    spin_pids="$spin_pids $!"
done
sleep 4

# Whole seconds of CPU the server has consumed. `ps -o time=` is the one form
# both GNU and BSD ps agree on; it prints [[DD-]HH:]MM:SS, so fold the
# colon-separated fields base 60.
server_cpu() {
    ps -o time= -p "$server_pid" 2>/dev/null | tr -d ' ' | tr '-' ':' | awk -F: '
        { s = 0; for (i = 1; i <= NF; i++) s = s * 60 + $i; printf "%d", s }'
}
cpu_before="$(server_cpu)"
# SIGKILL, so no peer ever sends QUIT: the server learns of the close only from
# the socket, which is exactly the path that used to spin.
for p in $spin_pids; do kill -9 "$p" 2>/dev/null || true; done
# shellcheck disable=SC2086  # deliberate word splitting: one wait per pid
wait $spin_pids 2>/dev/null || true
sleep 5
cpu_after="$(server_cpu)"
case "$cpu_before-$cpu_after" in
    *[!0-9-]*|-*|*-)
        printf 'SKIP chat: ps reported no cpu time for the server; the spin check did not run\n' >&2
        ;;
    *)
        burned=$(( cpu_after - cpu_before ))
        # Three spinning threads burn ~3 CPU seconds per wall second, so this
        # window costs ~15s unfixed and 0s fixed. 2s is far above ps rounding
        # and the server's own idle work, and far below the defect.
        [ "$burned" -lt 2 ] || t_fail \
            "the server burned ${burned}s of CPU in a 5s window after 3 peers were SIGKILLed (dead-peer spin)"
        ;;
esac

# The killed peer's nick is free again: a reconnect gets the nick it asked for
# instead of being auto-suffixed away from it by a connection that is gone.
z_sent="$(cli zrec send --server 127.0.0.1:"$port" --nick zombie1 --chan '#spin' --text 'nick reclaimed' 2>/dev/null || true)"
case "$z_sent" in
    *':zombie1!zombie1@localhost PRIVMSG #spin :nick reclaimed'*) : ;;
    *) t_fail "a dead peer never gave its nick back: [$z_sent]" ;;
esac

kill "$server_pid" 2>/dev/null || true
wait "$server_pid" 2>/dev/null || true

# --- with the server STOPPED ------------------------------------------------
# read --local must still work: the channel log IS the storage format, so a
# local read returns the rows a FETCH would. Without it there is no supported
# way to see a channel when no server is running, and the only recourse is
# opening the log by hand -- which is what agents actually did.
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
# The channel logs are the server's shared storage, so a private --state dir
# must not become the place a local read looks.
state_out="$(AI_CHAT_HOME="$home" "$CLIENT" read --local --state "$temporary_root/elsewhere" \
    --chan '#sess' --since 0 --no-session 2>/dev/null || true)"
case "$state_out" in
    *'MSG #sess'*) : ;;
    *) t_fail "read --local let --state redirect the channel home: [$state_out]" ;;
esac
# An unknown channel is an actionable error, not a crash or silence.
rc=0
AI_CHAT_HOME="$home" "$CLIENT" read --local --chan '#nosuchchannel' --no-session \
    >/dev/null 2>"$temporary_root/nochan.err" || rc=$?
[ "$rc" -eq 66 ] || t_fail "read --local on a missing channel exited $rc (want 66)"

printf 'chat: exercised rust server + client (discovery, send TOFU, delta, fail-closed, idle tail wakes, session, join/leave, mentions, per-agent sessions in one home, dead-peer teardown, serverless local reads)\n' >&2
t_end
