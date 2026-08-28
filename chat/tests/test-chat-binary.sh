#!/usr/bin/env bash
# MODE: DEV
# test-chat-binary.sh - the compiled `chat` binary: its startup contract and
# its interoperability with the bash helpers.
#
# What this pins, in the words of the requirement it comes from:
#   1. a second start on an endpoint that is already served declines
#   2. an explicit override exists, and it is refused unless it is complete
#   3. the override is explicit on both sides: a debug instance writes nothing
#      a default client reads, so no discovery path leads to it
#   4. bringing up a debug instance disturbs no other instance
# plus the thing that makes the binary worth shipping: the unmodified bash
# helpers talk to it, and it talks to the logs they write.
#
# EVERY instance started here names --bind and --port explicitly, so the suite
# can never contend with a chat bus that is actually in use on this machine.
# The default endpoint's own singleton behaviour is pinned by the crate's unit
# tests (`cargo test` in src/chat), which need no socket to prove it.
#
# Skipped, not failed, when the binary is not built: the toolchain is a dev
# dependency and a contributor without Rust must still be able to run the suite.

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
        fail "$1"
        printf '       expected: %s\n       actual:   %s\n' "$2" "$3"
    fi
}
contains() { # <description> <needle> <haystack>
    case "$3" in
        *"$2"*) pass "$1" ;;
        *) fail "$1"; printf '       wanted %s in:\n%s\n' "$2" "$3" ;;
    esac
}

binary=""
for candidate in "$crate/target/release/chat" "$crate/target/debug/chat"; do
    [ -x "$candidate" ] && { binary="$candidate"; break; }
done
if [ -z "$binary" ] && command -v cargo >/dev/null 2>&1; then
    printf '%s: building the binary (no prebuilt one found)\n' "$me"
    ( cd "$crate" && cargo build --quiet ) >/dev/null 2>&1
    [ -x "$crate/target/debug/chat" ] && binary="$crate/target/debug/chat"
fi
if [ -z "$binary" ]; then
    printf '%s: SKIP (no chat binary and no cargo to build one)\n' "$me"
    exit 0
fi

root="$(mktemp -d "${TMPDIR:-/tmp}/chat-bin.XXXXXX")"
pids=""
cleanup() {
    for p in $pids; do kill "$p" 2>/dev/null; done
    for pid_file in $(find "$root" -type f -name server.pid 2>/dev/null); do
        kill "$(cat "$pid_file" 2>/dev/null)" 2>/dev/null || true
    done
    # Give each server a moment to release its lock before the tree goes, so a
    # failure here is never mistaken for a lock defect on the next run.
    sleep 0.3
    rm -rf -- "$root"
}
trap cleanup EXIT INT TERM

# A port nothing is on. Asking the kernel for one and closing it would race, so
# this walks candidates and lets the server itself be the arbiter: the first
# port it actually binds is the one used.
#
# Readiness is the endpoint publication, not a display banner. The server writes
# this file only after its listener is bound, and the later client assertions
# verify that the published endpoint is usable. This keeps readiness independent
# of human-facing output and makes banner wording safe to change.
#
# Return codes: 0 ready (pid on stdout), 2 the port was taken (try another),
# 1 the server failed for some other reason, which is a real failure.
start_instance() { # <home> <port> -> prints the pid, or nothing
    local home="$1" port="$2" log="$1/serve.log" ready="$1/instances/127.0.0.1_${2}/server.port" i=0 pid
    mkdir -p "$home"
    : > "$log"
    "$binary" serve --home "$home" --no-register --bind 127.0.0.1 --port "$port" >"$log" 2>&1 &
    pid=$!
    while [ "$i" -lt 40 ]; do
        [ -f "$ready" ] && {
            cat "$home/instances/127.0.0.1_${port}/server.pid"
            return 0
        }
        case "$(cat "$log" "$home/server.log" 2>/dev/null)" in
            *"already in use"*) return 2 ;;
        esac
        if ! kill -0 "$pid" 2>/dev/null; then
            case "$(cat "$log" "$home/server.log" 2>/dev/null)" in
                *"already in use"*) return 2 ;;
            esac
        fi
        sleep 0.1
        i=$((i + 1))
    done
    printf '%s: the server never reported serving on %s. Its output:\n%s\n' \
        "$me" "$port" "$(cat "$log" 2>/dev/null)" >&2
    return 1
}

# Walk the candidate ports, distinguishing "taken" from "broken".
#
# Results come back in the globals found_port and found_pid rather than on
# stdout: a `x="$(find_port ...)"` would run this in a subshell, and under
# `set -u` the caller then dies on an unset found_port instead of reading it.
found_port=""
found_pid=""
find_port() { # <home> <port>... -> sets found_port and found_pid
    local home="$1" p rc
    shift
    found_port=""
    found_pid=""
    for p in "$@"; do
        found_pid="$(start_instance "$home" "$p")"; rc=$?
        case "$rc" in
            0) found_port="$p"; return 0 ;;
            # Not ours to keep: kill it before moving on. An earlier version
            # left one server per candidate port running, which then held those
            # ports and made the NEXT run skip — five leaked processes were
            # found by hand. Anything started here is killed here.
            2) kill "$found_pid" 2>/dev/null; rm -rf "$home" ;;
            *) kill "$found_pid" 2>/dev/null; return 1 ;;
        esac
    done
    return 2
}

find_port "$root/a" 17811 17823 17837 17849 17851; rc=$?
port_a="$found_port"
pid_a="$found_pid"
case "$rc" in
    0) pids="$pids $pid_a" ;;
    2) printf '%s: SKIP (no free loopback port among the candidates)\n' "$me"; exit 0 ;;
    *) printf '%s: FAIL (the server could not serve; see its output above)\n' "$me"; exit 1 ;;
esac
printf '%s: debug instance A on 127.0.0.1:%s\n' "$me" "$port_a"

# --- requirement 2: the override is refused unless it is complete -----------
out="$("$binary" serve --home "$root/z" --port 19999 2>&1)"; rc=$?
check "--port without --bind is a usage error" 64 "$rc"
contains "and it says which flag is missing" "needs --bind" "$out"
out="$("$binary" serve --home "$root/z" --bind 127.0.0.1 2>&1)"; rc=$?
check "--bind without --port is a usage error" 64 "$rc"
contains "and it says which flag is missing" "needs --port" "$out"

# --- requirement 1: a served endpoint declines a second start ---------------
out="$("$binary" serve --home "$root/a" --bind 127.0.0.1 --port "$port_a" 2>&1)"; rc=$?
check "a second start on a served endpoint declines" 69 "$rc"
contains "the refusal names the live port" "$port_a" "$out"

# A leftover pid file must not change that answer in either direction: the
# whole point of deciding from a lock is that a file cannot lie about it.
printf '999999\n' > "$root/a/instances/127.0.0.1_${port_a}/server.pid"
out="$("$binary" serve --home "$root/a" --bind 127.0.0.1 --port "$port_a" 2>&1)"; rc=$?
check "still declines when the pid file names a dead process" 69 "$rc"

# --- requirement 3: nothing a default client reads points at a debug bus ----
if [ -e "$root/a/server.port" ]; then
    fail "a debug instance must not write <home>/server.port"
else
    pass "a debug instance does not write <home>/server.port"
fi
if [ -e "$root/a/server.pid" ]; then
    fail "a debug instance must not write <home>/server.pid"
else
    pass "a debug instance does not write <home>/server.pid"
fi
check "it publishes under its own endpoint directory" "$port_a" \
    "$(sed -n '1p' "$root/a/instances/127.0.0.1_$port_a/server.port")"

# --- the clients, binary and bash, against the same bus ---------------------
"$binary" register '#t' --host 127.0.0.1 --port "$port_a" >/dev/null 2>&1
stored="$("$binary" send '#t' 'from the binary' -n binclient --host 127.0.0.1 --port "$port_a" 2>&1)"
contains "the binary client stores a message over the socket" "MSG #t 1 " "$stored"

helper="$("$BASH" "$scripts/chat-send.sh" '#t' 'from the helper' -n bashclient \
    --host 127.0.0.1 --port "$port_a" 2>&1)"
contains "an unmodified chat-send.sh stores against the binary" "MSG #t 2 " "$helper"

rows="$("$BASH" "$scripts/chat-read.sh" '#t' --host 127.0.0.1 --port "$port_a" 2>&1)"
contains "chat-read.sh reads both back over the socket" "from the binary" "$rows"
contains "chat-read.sh reads both back over the socket (2)" "from the helper" "$rows"

rows="$("$binary" read '#t' --host 127.0.0.1 --port "$port_a" 2>&1)"
check "the binary reads the same two rows remotely" 2 "$(printf '%s\n' "$rows" | wc -l | tr -d ' ')"

# No --host: the client must not touch a socket at all, and must still work.
rows="$("$binary" read '#t' --local --home "$root/a" 2>&1)"
check "a client with no --host reads the log directly" 2 \
    "$(printf '%s\n' "$rows" | wc -l | tr -d ' ')"
contains "and a local send lands in the same log" "MSG #t 3 " \
    "$("$binary" send '#t' 'local, no server' -n local --home "$root/a" 2>&1)"

# --- requirement 4: a second debug instance disturbs nothing ----------------
find_port "$root/b" 17863 17879 17881 17891 && pids="$pids $found_pid"
port_b="$found_port"
if [ -z "$port_b" ]; then
    fail "could not start a second debug instance to test isolation"
else
    printf '%s: debug instance B on 127.0.0.1:%s\n' "$me" "$port_b"
    check "instance A is still serving its own port" "$port_a" \
        "$(sed -n '1p' "$root/a/instances/127.0.0.1_$port_a/server.port")"
    if kill -0 "$pid_a" 2>/dev/null; then
        pass "instance A's process is untouched by B starting"
    else
        fail "instance A died when B started"
    fi
    stored="$("$binary" send '#t' 'only on B' -n bclient --host 127.0.0.1 --port "$port_b" 2>&1)"
    contains "B stores into its own store, starting at id 1" "MSG #t 1 " "$stored"
    a_rows="$("$binary" read '#t' --home "$root/a" 2>&1)"
    case "$a_rows" in
        *"only on B"*) fail "B's message leaked into A's store" ;;
        *) pass "B's message is absent from A's store" ;;
    esac
    stored="$("$binary" send '#t' 'still A' -n aclient --host 127.0.0.1 --port "$port_a" 2>&1)"
    contains "A still accepts traffic after B started" "MSG #t 4 " "$stored"
fi

if [ "$fails" -eq 0 ]; then
    printf '%s: PASS\n' "$me"
    exit 0
fi
printf '%s: FAIL (%s)\n' "$me" "$fails"
exit 1
