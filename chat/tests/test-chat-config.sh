#!/usr/bin/env bash
# MODE: DEV
# test-chat-config.sh - the transport decision: asked once, recorded, and read
# by both the server and the clients.
#
# What this pins:
#   1. a first run with no config asks, and records exactly what was chosen
#   2. a second run does not ask
#   3. a run with no tty neither asks nor writes - the important one, because a
#      default nobody chose, in a file that then looks like a decision, is worse
#      than not asking
#   4. an explicit flag overrides a config that says otherwise
#   5. a debug instance leaves the config byte-identical
#   6. a client with no flags reaches the server the config describes, for each
#      transport, and that includes the unmodified bash helpers
#   7. an unreachable configured server falls back to the log, and says so
#
# Where a config is written by hand rather than through the prompt, it is because
# the prompt's two tcp options both bind 7717 and this suite must never contend
# with a bus in real use. The file documents that it may be hand-edited, so this
# is a supported path and not a test-only back door.
#
# Skipped, not failed, when the binary is not built: the toolchain is a dev
# dependency and a contributor without Rust must still run the suite.

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
check() { # <description> <expected> <actual>
    if [ "$2" = "$3" ]; then pass "$1"; else
        fail "$1"; printf '       expected: %s\n       actual:   %s\n' "$2" "$3"
    fi
}
contains() { # <description> <needle> <haystack>
    case "$3" in
        *"$2"*) pass "$1" ;;
        *) fail "$1"; printf '       wanted %s in:\n%s\n' "$2" "$3" ;;
    esac
}
absent() { # <description> <path>
    if [ -e "$2" ]; then fail "$1"; printf '       exists: %s\n' "$2"; else pass "$1"; fi
}

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

# Short root: a unix socket path has a kernel cap near 104 bytes, so a long
# scratch directory makes the socket transport untestable rather than failing it.
root="$(mktemp -d /tmp/ch-cfg.XXXXXX)"
pids=""
cleanup() {
    for p in $pids; do kill "$p" 2>/dev/null; done
    sleep 0.3
    rm -rf -- "$root"
}
trap cleanup EXIT INT TERM

# A pty, so the prompt is reachable at all. util-linux and BSD `script` disagree
# on argument order, so both are tried and the tty checks are skipped rather than
# failed where neither fits.
pty_style=""
if command -v script >/dev/null 2>&1; then
    if script -q -c true /dev/null >/dev/null 2>&1; then
        pty_style=gnu
    elif script -q /dev/null true >/dev/null 2>&1; then
        pty_style=bsd
    fi
fi
answer_prompt() { # <answer> <home> -> runs serve on a pty, briefly
    local answer="$1" home="$2"
    case "$pty_style" in
        gnu) printf '%s\n' "$answer" | timeout 6 script -q -c "'$binary' serve --home '$home'" /dev/null >"$home/tty.out" 2>&1 ;;
        bsd) printf '%s\n' "$answer" | timeout 6 script -q /dev/null "$binary" serve --home "$home" >"$home/tty.out" 2>&1 ;;
    esac
    return 0
}

# --- 3: no tty neither asks nor writes --------------------------------------
h="$root/notty"; mkdir -p "$h"
timeout 4 "$binary" serve --home "$h" >"$h/out" 2>&1
out="$(cat "$h/out")"
contains "a run with no tty says there was no terminal to ask" "no terminal to ask" "$out"
contains "and says nothing was recorded" "nothing has been recorded" "$out"
contains "and names the transport it used for the run" "127.0.0.1:7717" "$out"
absent "a transport nobody chose is not recorded" "$h/config"
check "chat config reports nothing recorded" 1 \
    "$("$binary" config --home "$h" >/dev/null 2>&1; echo $?)"

# --- 1 and 2: asked once, recorded, not asked again -------------------------
if [ -z "$pty_style" ]; then
    printf '%s: no usable `script`, so the interactive prompt is not exercised\n' "$me"
else
    for answer in 1 2 3; do
        h="$root/tty$answer"; mkdir -p "$h"
        answer_prompt "$answer" "$h"
        if [ ! -f "$h/config" ]; then
            fail "answering $answer records a config"
            continue
        fi
        recorded="$(sed -n 's/^transport=//p' "$h/config")"
        bind="$(sed -n 's/^bind=//p' "$h/config")"
        case "$answer" in
            1) check "answering 1 records a unix socket" socket "$recorded" ;;
            2) check "answering 2 records tcp on loopback" "tcp 127.0.0.1" "$recorded $bind" ;;
            3) check "answering 3 records tcp on every interface" "tcp 0.0.0.0" "$recorded $bind" ;;
        esac
        contains "the prompt states what 0.0.0.0 exposes" "post as any nick" "$(cat "$h/tty.out")"
        contains "the prompt says it is asked once" "asked once" "$(cat "$h/tty.out")"
    done
    # Asked once means once: a second run must not re-ask even with a terminal.
    h="$root/tty2"
    before="$(cat "$h/config")"
    answer_prompt 1 "$h"
    check "a second run does not ask again" "$before" "$(cat "$h/config")"
fi

# --- 6 and 7: the clients read it ------------------------------------------
# A high port, by hand, so this never contends with a real bus on 7717.
h="$root/tcp"; mkdir -p "$h"
printf 'transport=tcp\nbind=127.0.0.1\nport=18333\n' > "$h/config"

echo "  -- with the configured server down --"
out="$("$binary" send '#t' 'down' -n a --home "$h" 2>&1)"
contains "the binary client falls back to the log and says so" "falling back to the log" "$out"
contains "and the message is still stored" "MSG #t 1 " "$out"
out="$("$BASH" "$scripts/chat-send.sh" '#t' 'down too' -n b --home "$h" 2>&1)"
contains "a bash helper falls back to the log and says so" "using the log directly" "$out"
contains "and its message is still stored" "MSG #t 2 " "$out"

echo "  -- with the configured server up --"
"$binary" serve --home "$h" >"$h/serve.log" 2>&1 &
pids="$pids $!"
i=0; while [ "$i" -lt 40 ]; do
    [ -f "$h/server.port" ] && break
    sleep 0.1; i=$((i + 1))
done
contains "the server serves the configured endpoint" "127.0.0.1:18333" "$(cat "$h/serve.log")"
contains "and says the transport came from the config" "from the recorded config" "$(cat "$h/serve.log")"
out="$("$binary" send '#t' 'via config' -n c --home "$h" 2>&1)"
contains "a flagless binary client reaches it, with no fallback note" "MSG #t 3 " "$out"
case "$out" in *"falling back"*) fail "it fell back although the server was up" ;; *) pass "no fallback while the server is up" ;; esac
out="$("$BASH" "$scripts/chat-send.sh" '#t' 'bash via config' -n d --home "$h" 2>&1)"
contains "a flagless bash helper reaches it too" "MSG #t 4 " "$out"
case "$out" in *"using the log directly"*) fail "the helper fell back although the server was up" ;; *) pass "the helper did not fall back" ;; esac

# --- 4: an explicit flag beats the config ----------------------------------
out="$("$binary" send '#t' 'explicit elsewhere' -n e --home "$h" --host 127.0.0.1 --port 1 2>&1)"
contains "an explicit --host/--port overrides the config" "127.0.0.1:1" "$out"

# --- 5: a debug instance leaves the config byte-identical ------------------
before="$(cat "$h/config")"
"$binary" serve --home "$h" --bind 127.0.0.1 --port 18334 >"$h/dbg.log" 2>&1 &
pids="$pids $!"
sleep 1
contains "a debug instance starts alongside the configured bus" "debug instance" "$(cat "$h/dbg.log")"
check "and leaves the config byte-identical" "$before" "$(cat "$h/config")"
absent "and writes no default endpoint advert of its own" "$h/instances/127.0.0.1_18334/config"

# --- 6, socket transport ---------------------------------------------------
h="$root/sock"; mkdir -p "$h"
printf 'transport=socket\nsocket=%s/chat.sock\n' "$h" > "$h/config"
"$binary" serve --home "$h" >"$h/serve.log" 2>&1 &
pids="$pids $!"
i=0; while [ "$i" -lt 40 ]; do
    [ -f "$h/server.socket" ] && break
    sleep 0.1; i=$((i + 1))
done
if [ ! -S "$h/chat.sock" ]; then
    fail "the socket transport did not produce a socket"
    printf '       server said: %s\n' "$(cat "$h/serve.log")"
else
    pass "the socket transport binds a unix socket"
    absent "a socket bus writes no port file for a client to dial" "$h/server.port"
    check "and records its path where a port would go" "$h/chat.sock" \
        "$(sed -n '1p' "$h/server.socket")"
    out="$("$binary" send '#t' 'over the socket' -n f --home "$h" 2>&1)"
    contains "a flagless binary client reaches the unix socket" "MSG #t 1 " "$out"
    case "$out" in *"falling back"*) fail "it fell back although the socket was up" ;; *) pass "no fallback over the socket" ;; esac
    # bash has no unix-domain /dev/tcp, so the helper must degrade, and say why.
    out="$("$BASH" "$scripts/chat-send.sh" '#t' 'bash cannot' -n g --home "$h" 2>&1)"
    contains "a bash helper says bash cannot open a unix socket" "bash cannot open" "$out"
    contains "and still stores the message locally" "MSG #t 2 " "$out"
fi

if [ "$fails" -eq 0 ]; then
    printf '%s: PASS\n' "$me"
    exit 0
fi
printf '%s: FAIL (%s)\n' "$me" "$fails"
exit 1
