#!/usr/bin/env bash
# MODE: DEV
# test-chat.sh - the chat skill, across every runtime this machine offers.
#
# For each available runtime (python3/node/perl/socat): start the real
# server, exercise register/send/read over the socket and locally, the delta
# (--since / FETCH), join-push to a second connection, persistence across a
# restart, and the refusals. A missing runtime is SKIP, not failure; if none
# exists the suite still covers direct-log operation.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scripts="$root/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")/../../planning/tests" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

fail() { t_fail "$*"; }
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/chat-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

runtimes=""
for r in python3 node perl socat; do
    command -v "$r" >/dev/null 2>&1 && runtimes="$runtimes $r"
done
[ -n "$runtimes" ] || { printf 'chat: no server runtime present; log-only coverage\n'; }

port_for() { # <home>
    cat "$1/server.port"
}

wait_quiet_port_free() { # <home> — after stop, the pid is gone; nothing more needed.
    :
}

exercise_runtime() { # <runtime>
    local rt="$1" home port rc a b line
    home="$temporary_root/home-$rt"
    rm -rf "$home"

    local extra=()
    [ "$rt" = socat ] && extra=(--port 47931)

    rc=0
    "$scripts/chat-server.sh" start --runtime "$rt" "${extra[@]+"${extra[@]}"}" --home "$home" \
        >"$temporary_root/start.$rt.log" 2>&1 || rc=$?
    if [ "$rc" -ne 0 ]; then
        fail "$rt: server start failed (rc=$rc): $(cat "$temporary_root/start.$rt.log")"
        return 0
    fi
    port="$(port_for "$home")"
    case "$port" in ''|*[!0-9]*) fail "$rt: bad port file: $port"; return 0 ;; esac

    # register + local send + local read (no server involvement on the wire)
    "$scripts/chat-register.sh" '#ops' --home "$home" >/dev/null
    a="$("$scripts/chat-send.sh" '#ops' 'local one' -n localnick --home "$home")"
    case "$a" in MSG\ \#ops\ 2\ *localnick\ :local\ one) : ;; *) fail "$rt: local send produced [$a]" ;; esac
    b="$("$scripts/chat-read.sh" '#ops' --since 1 --home "$home")"
    [ "$b" = "$a" ] || fail "$rt: local --since read mismatch [$b]"

    # socket send echoes the stored line; socket fetch agrees
    a="$("$scripts/chat-send.sh" '#ops' 'remote two' -n remnick --host 127.0.0.1 --port "$port")"
    case "$a" in MSG\ \#ops\ 3\ *) : ;; *) fail "$rt: remote send produced [$a]" ;; esac
    b="$("$scripts/chat-read.sh" '#ops' --since 0 --host 127.0.0.1 --port "$port" | tail -1)"
    [ "$b" = "$a" ] || fail "$rt: socket FETCH mismatch [$b] vs [$a]"

    # delta excludes older ids
    b="$("$scripts/chat-read.sh" '#ops' --since 2 --host 127.0.0.1 --port "$port")"
    case "$b" in *local\ one*|*registered*) fail "$rt: --since leaked old ids: [$b]" ;; esac

    # join semantics: python3/node push live; perl/socat answer poll mode and
    # clients receive via FETCH/tail instead.
    (
        exec 3<> /dev/tcp/127.0.0.1/"$port"
        printf 'NICK watcher\nJOIN #ops\n' >&3
        # No timeout(1) on macOS (B6): read -t bounds each line-wait, and the
        # loop stops once this client's expected reply arrives or ~5s lapse.
        chat_deadline=$(( SECONDS + 5 ))
        while [ "$SECONDS" -lt "$chat_deadline" ]; do
            if IFS= read -t 1 -r chat_line <&3; then
                printf '%s\n' "$chat_line" >> "$temporary_root/push.$rt"
                case "$chat_line" in
                    *pusher\ :pushed*|*poll\ mode*) break ;;
                esac
            fi
        done
    ) &
    pusher=$!
    sleep 1
    "$scripts/chat-send.sh" '#ops' 'pushed' -n pusher --host 127.0.0.1 --port "$port" >/dev/null
    wait "$pusher" 2>/dev/null || true
    case "$rt" in
        python3|node)
            grep -q 'MSG #ops .*pusher :pushed' "$temporary_root/push.$rt" \
                || fail "$rt: JOIN did not push the next message" ;;
        *)
            grep -q 'OK join .*poll mode' "$temporary_root/push.$rt" \
                || fail "$rt: poll-mode JOIN not acknowledged" ;;
    esac

    # persistence: restart, history survives
    "$scripts/chat-server.sh" stop --home "$home" >/dev/null
    rc=0
    "$scripts/chat-server.sh" start --runtime "$rt" "${extra[@]+"${extra[@]}"}" --home "$home" \
        >/dev/null 2>&1 || rc=$?
    if [ $rt = socat ]; then
        # socat rebinds its fixed port; give the fork handler a beat
        sleep 0.5
    fi
    [ "$rc" -eq 0 ] || fail "$rt: restart after stop failed (rc=$rc)"
    b="$("$scripts/chat-read.sh" '#ops' --last 1 --home "$home")"
    case "$b" in *':pushed') : ;; *) fail "$rt: history lost after restart: [$b]" ;; esac

    # unknown channel reads refuse with 66
    rc=0
    "$scripts/chat-read.sh" '#nosuch' --home "$home" >/dev/null 2>&1 || rc=$?
    [ "$rc" -eq 66 ] || fail "$rt: reading an absent channel exited $rc, want 66"

    "$scripts/chat-server.sh" stop --home "$home" >/dev/null
    printf 'chat: exercised %s\n' "$rt"
}

for rt in $runtimes; do
    exercise_runtime "$rt"
done

# --- no-server path still works end to end -----------------------------------
home="$temporary_root/logonly"
rm -rf "$home"
"$scripts/chat-register.sh" '#solo' --home "$home" >/dev/null
"$scripts/chat-send.sh" '#solo' 'without any server' -n ghost --home "$home" >/dev/null
line="$("$scripts/chat-read.sh" '#solo' --last 1 --home "$home")"
case "$line" in *ghost\ :without\ any\ server) : ;; *) fail "log-only flow broken: [$line]" ;; esac

# --- the bind address plumbs to every runtime (B30) ---------------------------
# Each tier reads AI_CHAT_BIND for its listening socket, and the launcher
# records what it bound. A non-loopback functional bind depends on interfaces
# this machine may not have, so the cross-machine case is documented in
# SKILL.md rather than automated here.
for bind_site in \
    "$scripts/chat-server.sh" \
    "$root/runtime/server.py" \
    "$root/runtime/server.js" \
    "$root/runtime/server.pl"; do
    grep -q 'AI_CHAT_BIND' "$bind_site" || fail "$(basename "$bind_site") ignores AI_CHAT_BIND"
done
bind_home="$temporary_root/bindhome"
if "$scripts/chat-server.sh" start --runtime python3 --port 18471 --bind 127.0.0.1 --home "$bind_home" >/dev/null 2>&1; then
    [ "$(cat "$bind_home/server.bind")" = "127.0.0.1" ] || fail "server.bind did not record the bind address"
    "$scripts/chat-send.sh" '#b' 'bound roundtrip' -n binder --host 127.0.0.1 --port 18471 --home "$bind_home" >/dev/null 2>&1 \
        || fail "explicit --bind broke the send path"
    line="$("$scripts/chat-read.sh" '#b' --last 1 --host 127.0.0.1 --port 18471 --home "$bind_home")"
    case "$line" in *bound\ roundtrip*) : ;; *) fail "explicit --bind broke delivery: [$line]" ;; esac
    "$scripts/chat-server.sh" stop --home "$bind_home" >/dev/null 2>&1 || true
fi

t_end
