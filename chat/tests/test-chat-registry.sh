#!/usr/bin/env bash
# MODE: DEV
# test-chat-registry.sh - servers register, chatters discover, and a started
# server outlives the shell that started it.
#
# What this pins:
#   1. the registry directory is per-uid and 0700, because /tmp is world-writable
#   2. a chatter that finds no server starts one and adopts it
#   3. that server is in its own session with no controlling terminal, so closing
#      the shell does not take it down
#   4. later chatters attach to it rather than standing up a second
#   5. many chatters starting at once produce exactly ONE server, and every
#      message lands - the case auto-start makes likely rather than theoretical
#   6. `chat stop` ends it and evicts the entry itself
#   7. an entry is not proof of life: a stale one is evicted and the caller
#      behaves as though it had never been there
#   8. the bash helpers discover through the same registry, and prefer it to the
#      config - the config says how a bus should be exposed, the registry says
#      where one actually is
#   9. a debug instance registers nothing, so discovery cannot land on it
#
# XDG_RUNTIME_DIR is set per case, which is the documented location and therefore
# the real resolution path - not a test-only hook. It also keeps every case out of
# the developer's own registry, and out of every other case's.
#
# Skipped, not failed, when the binary is not built.

set -uo pipefail
export LC_ALL=C

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$here/../.." && pwd)"
scripts="$repo_root/chat/scripts"
crate="$repo_root/src/chat"
me="${0##*/}"

fails=0
pass() { printf '  ok   %s\n' "$1"; }
fail() { printf '  FAIL %s\n' "$1"; fails=$((fails + 1)); }
check() { if [ "$2" = "$3" ]; then pass "$1"; else fail "$1"; printf '       expected: %s\n       actual:   %s\n' "$2" "$3"; fi; }
contains() { case "$3" in *"$2"*) pass "$1" ;; *) fail "$1"; printf '       wanted %s in:\n%s\n' "$2" "$3" ;; esac; }

binary=""
for candidate in "$crate/target/release/chat" "$crate/target/debug/chat"; do
    [ -x "$candidate" ] && { binary="$candidate"; break; }
done
if [ -z "$binary" ] && command -v cargo >/dev/null 2>&1; then
    ( cd "$crate" && cargo build --quiet ) >/dev/null 2>&1
    [ -x "$crate/target/debug/chat" ] && binary="$crate/target/debug/chat"
fi
if [ -z "$binary" ]; then
    printf '%s: SKIP (no chat binary and no cargo to build one)\n' "$me"
    exit 0
fi

# Short root: a unix socket path has a kernel cap near 104 bytes.
root="$(mktemp -d /tmp/ch-reg.XXXXXX)"
homes=""
cleanup() {
    # Stop every server this test started, through the same path a user would,
    # so a failed run does not leave daemons behind. They are detached, so
    # nothing else will reap them.
    for h in $homes; do
        XDG_RUNTIME_DIR="$root/run" "$binary" stop --home "$h" >/dev/null 2>&1
    done
    pkill -f "chat serve --foreground .*$root" 2>/dev/null
    sleep 0.3
    rm -rf -- "$root"
}
trap cleanup EXIT INT TERM

new_home() { # <tag> <port> -> prints the home
    local h="$root/$1"
    mkdir -p "$h"
    printf 'transport=tcp\nbind=127.0.0.1\nport=%s\n' "$2" > "$h/config"
    homes="$homes $h"
    printf '%s\n' "$h"
}

export XDG_RUNTIME_DIR="$root/run"
mkdir -p "$XDG_RUNTIME_DIR"
regdir="$XDG_RUNTIME_DIR/chat"
entries() { ls -1 "$regdir"/*.server 2>/dev/null | wc -l | tr -d ' '; }

# --- 2: auto-start ----------------------------------------------------------
h="$(new_home autostart 18501)"
out="$("$binary" send '#t' 'first ever' -n first --home "$h" 2>&1)"
contains "a chatter with no server starts one" "so one was started" "$out"
contains "and says where it registered" "$regdir" "$out"
contains "and the message is stored" "MSG #t 1 " "$out"
check "exactly one server is registered" 1 "$(entries)"

# --- 1: the directory is private -------------------------------------------
# `ls -ld`, not `stat -c`: stat's format flag is -c on GNU and -f on BSD, which
# PORTABILITY.md bans outright rather than guarded, and the permission string
# says what matters more legibly than an octal mode anyway. The substring drops
# any trailing '.' or '+' that SELinux or an ACL appends.
perms="$(ls -ld "$regdir" | awk '{print substr($1, 1, 10)}')"
check "the registry directory is private (/tmp is world-writable)" "drwx------" "$perms"

# --- 3: it survives its starting shell -------------------------------------
pid="$(sed -n 's/^pid=//p' "$regdir"/*.server | sed -n '1p')"
if [ -z "$pid" ]; then
    fail "the entry records a pid"
else
    sid="$(ps -o sid= -p "$pid" 2>/dev/null | tr -d ' ')"
    tty="$(ps -o tty= -p "$pid" 2>/dev/null | tr -d ' ')"
    check "the server leads its own session, so SIGHUP does not reach it" "$pid" "$sid"
    check "and has no controlling terminal" "?" "$tty"
fi

# --- 4: later chatters attach ----------------------------------------------
out="$("$binary" send '#t' 'second' -n second --home "$h" 2>&1)"
case "$out" in
    *"so one was started"*) fail "a second chatter must attach, not start another" ;;
    *) pass "a second chatter attaches silently" ;;
esac
check "still exactly one server" 1 "$(entries)"
out="$("$binary" serve --home "$h" 2>&1)"
contains "chat serve reports the running one instead of starting another" "already running" "$out"

# --- 8: the helpers discover through the registry --------------------------
# The config is made deliberately wrong: if the helper reaches the server anyway,
# it discovered rather than assumed.
printf 'transport=tcp\nbind=127.0.0.1\nport=19998\n' > "$h/config"
before="$(wc -l < "$h/channels/#t.log" | tr -d ' ')"
out="$("$BASH" "$scripts/chat-send.sh" '#t' 'via registry' -n helper --home "$h" 2>&1)"
contains "a bash helper reaches the registered server, not the config's port" \
    "MSG #t $((before + 1)) " "$out"
case "$out" in
    *"nothing answers"*) fail "the helper used the config port instead of the registry" ;;
    *) pass "and did not fall back" ;;
esac

# --- 9: a debug instance registers nothing ---------------------------------
before="$(entries)"
"$binary" serve --home "$h" --bind 127.0.0.1 --port 18502 --no-register >/dev/null 2>&1
sleep 1
check "a --no-register instance adds no registry entry" "$before" "$(entries)"

# --- 6: stop ---------------------------------------------------------------
out="$("$binary" stop --home "$h" 2>&1)"
contains "stop reports what it stopped" "stopped:" "$out"
check "and evicts the entry itself" 0 "$(entries)"
if kill -0 "$pid" 2>/dev/null; then fail "the server process is gone after stop"; else pass "the server process is gone after stop"; fi
"$binary" stop --home "$h" >/dev/null 2>&1
check "stopping nothing exits 1 rather than erroring" 1 "$?"

# --- 7: an entry is not proof of life -------------------------------------
h2="$(new_home stale 18503)"
mkdir -p "$regdir"
# Port 1: privileged and nothing listens there, which is the crashed-server case.
printf 'home=%s\npid=999999\nstarted=1\ntransport=tcp\nbind=127.0.0.1\nport=1\n' "$h2" \
    > "$regdir/1-999999.server"
check "the planted stale entry is present to begin with" 1 "$(entries)"
out="$("$binary" servers 2>&1)"
contains "chat servers marks it stale rather than deleting it" "stale" "$out"
out="$("$binary" send '#t' 'after a crash' -n post --home "$h2" 2>&1)"
contains "a client evicts the corpse and starts a server instead" "so one was started" "$out"
contains "and the message lands" "MSG #t 1 " "$out"
check "and only the new entry remains" 1 "$(entries)"
"$binary" stop --home "$h2" >/dev/null 2>&1

# --- 5: the race ----------------------------------------------------------
h3="$(new_home race 18504)"
for i in 1 2 3 4 5 6; do
    "$binary" send '#race' "m$i" -n "a$i" --home "$h3" >/dev/null 2>"$h3/err.$i" &
done
wait
check "six chatters at once produce exactly one server" 1 "$(entries)"
check "and every message is stored" 6 "$(wc -l < "$h3/channels/#race.log" | tr -d ' ')"
check "with contiguous ids, so no id was allocated twice" "1 2 3 4 5 6" \
    "$(awk '{print $3}' "$h3/channels/#race.log" | sort -n | tr '\n' ' ' | sed 's/ $//')"
started=0; attached=0
for i in 1 2 3 4 5 6; do
    case "$(cat "$h3/err.$i")" in
        *"another chatter started one at the same moment"*) attached=$((attached + 1)) ;;
        *"so one was started"*) started=$((started + 1)) ;;
    esac
done
# The loser must not claim to have started the server. Eight "started" lines for
# one server is a log that misleads whoever reads it later.
check "exactly one chatter reports starting the server" 1 "$started"
check "and the rest report attaching to it" 5 "$attached"
"$binary" stop --home "$h3" >/dev/null 2>&1

if [ "$fails" -eq 0 ]; then
    printf '%s: PASS\n' "$me"
    exit 0
fi
printf '%s: FAIL (%s)\n' "$me" "$fails"
exit 1
