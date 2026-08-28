#!/usr/bin/env bash
# MODE: PROD
# chat-server.sh - start/stop/status the chat socket server, choosing the
# first available runtime: python3, node, perl, or socat driving the bash
# handler (poll mode). State lives under $AI_CHAT_HOME (default ~/.ai-chat).
#
# Usage:
#   chat-server.sh start [--runtime R] [--port N] [--bind ADDR] [--home D]
#   chat-server.sh status [--home D]
#   chat-server.sh stop   [--home D]
#   chat-server.sh --help
#
# start waits until the chosen port is written to <home>/server.port and then
# prints it. --bind sets the listening address (default 127.0.0.1); a
# non-loopback bind exposes an unauthenticated protocol to its network, so set
# it only where that is the point.
# Exit codes: 64 bad invocation, 66 runtime missing, 69 no runtime at all, 70 internal.

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} start [--runtime python3|node|perl|socat|chat-server-rs]
                     [--port N] [--bind ADDR] [--home D]
                     [--announce [NAME]]  discoverable via chat-discover.sh
       ${0##*/} status [--home D]
       ${0##*/} stop   [--home D]

The runtime chain falls back through what is installed; --runtime pins one.
--bind defaults to 127.0.0.1; a non-loopback address exposes the unauthenticated
protocol to that network.
Without any of them the server cannot open a socket (exit 69) while every
client helper keeps working against existing logs.
USAGE
    exit "$rc"
}

HOME_DIR="${AI_CHAT_HOME:-$HOME/.ai-chat}"
cmd="" want_runtime="" want_port="" want_bind=""
announce=false announce_name=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        start|stop|status) [ -z "$cmd" ] || usage; cmd="$1"; shift ;;
        --runtime)
            [ "$#" -ge 2 ] || usage
            # B54: an unvalidated value is executed below; only the four
            # tiers exist, and anything else must be a usage error.
            case "$2" in
                python3|node|perl|socat|chat-server-rs) ;;
                *) printf '%s: unknown runtime: %s (python3, node, perl, socat)\n' "${0##*/}" "$2" >&2
                   exit 64 ;;
            esac
            want_runtime="$2"; shift 2 ;;
        --port) [ "$#" -ge 2 ] || usage; want_port="$2"; shift 2 ;;
        --bind) [ "$#" -ge 2 ] || usage; want_bind="$2"; shift 2 ;;
        --home) [ "$#" -ge 2 ] || usage; HOME_DIR="$2"; shift 2 ;;
        --announce)
            announce=true
            # optional human name: --announce my-name (a following --flag means unnamed)
            if [ "$#" -ge 2 ] && case "$2" in --*) false ;; *) true ;; esac; then
                announce_name="$2"; shift
            fi
            shift ;;
        *) usage ;;
    esac
done
[ -n "$cmd" ] || usage
case "$want_port" in ''|*[!0-9]*) want_port="" ;; esac
# Conservative address charset (IPv4/IPv6/hostname) when set — this string
# reaches a socat option and three interpreters.
case "${want_bind:-}" in
    ''|*[!A-Za-z0-9.:_-]*) [ -z "${want_bind:-}" ] || usage ;;
esac
AI_CHAT_BIND="${want_bind:-127.0.0.1}"
export AI_CHAT_BIND

CHAN_DIR="$HOME_DIR/channels"
RUN_DIR="$HOME_DIR/run"
pidfile="$HOME_DIR/server.pid"
portfile="$HOME_DIR/server.port"

is_running() {
    [ -f "$pidfile" ] || return 1
    kill -0 "$(cat "$pidfile")" 2>/dev/null
}

# The compiled rung lives beside the interpreter tiers when a build has put
# it there (cargo build --release && cp src/chat-server-rs/target/release/
# chat-server-rs chat/runtime/), or anywhere on PATH.
rust_bin() {
    if command -v chat-server-rs >/dev/null 2>&1; then
        command -v chat-server-rs
        return 0
    fi
    local cand
    cand="$(cd "$(dirname "${BASH_SOURCE[0]}")/../runtime" 2>/dev/null && pwd)/chat-server-rs"
    if [ -x "$cand" ]; then
        printf '%s\n' "$cand"
        return 0
    fi
    return 1
}

pick_runtime() {
    # A resident compiled server answers without interpreter startup, so it
    # leads the chain; absent it, the interpreter tiers follow unchanged.
    if [ -z "$want_runtime" ] || [ "$want_runtime" = chat-server-rs ]; then
        if [ -z "$want_runtime" ] && rust_bin >/dev/null; then
            printf 'chat-server-rs\n'
            return 0
        fi
        [ "$want_runtime" = chat-server-rs ] && { rust_bin; return $?; }
    fi
    local order="${want_runtime:-python3 node perl socat}"
    for r in $order; do
        command -v "$r" >/dev/null 2>&1 && { printf '%s\n' "$r"; return 0; }
    done
    return 1
}


# T65: launch the discovery beacon beside the server; stop kills it by pid.
start_announce_beacon() {
    announce_args=(--port "$(cat "$portfile")" --home "$HOME_DIR")
    if [ -n "$announce_name" ]; then
        announce_args=(--name "$announce_name" "${announce_args[@]}")
    fi
    nohup "$(dirname "${BASH_SOURCE[0]}")/chat-announce.sh" "${announce_args[@]}" \
        >> "$HOME_DIR/server.log" 2>&1 &
    echo $! > "$HOME_DIR/announce.pid"
    printf '%s: announcing as %s (chat-discover.sh finds it)\n' \
        "${0##*/}" "${announce_name:-auto}" >&2
}


# Stage the chosen runtime's files and start it under nohup; one shape per
# tier (the compiled rung needs no staging, socat gets none either).
launch_runtime() {
    local script src
    case "$runtime" in
        python3) src=server.py ;;
        node) src=server.js ;;
        perl) src=server.pl ;;
        chat-server-rs|socat) src="" ;;
    esac
    case "$runtime" in
        python3|node|perl)
            script="$RUN_DIR/$src"
            cp "$(dirname "${BASH_SOURCE[0]}")/../runtime/$src" "$script" ;;
        chat-server-rs|socat)
            script="" ;;
    esac

    : > "$HOME_DIR/server.log"
    printf '%s\n' "$AI_CHAT_BIND" > "$HOME_DIR/server.bind"
    if [ "$runtime" = socat ]; then
        handler="$(dirname "${BASH_SOURCE[0]}")/../runtime/bash-handler.sh"
        AI_CHAT_HOME="$HOME_DIR" nohup socat "TCP-LISTEN:${want_port:-0},fork,reuseaddr,bind=$AI_CHAT_BIND" \
            "EXEC:$handler" >> "$HOME_DIR/server.log" 2>&1 &
    elif [ "$runtime" = chat-server-rs ]; then
        AI_CHAT_HOME="$HOME_DIR" nohup "$(rust_bin)" ${want_port:+$want_port} \
            >> "$HOME_DIR/server.log" 2>&1 &
    else
        AI_CHAT_HOME="$HOME_DIR" nohup ${runtime} ${script:+$script} ${want_port:+$want_port} \
            >> "$HOME_DIR/server.log" 2>&1 &
    fi
    echo $! > "$pidfile"
    if [ "$runtime" = socat ]; then printf '%s\n' "$want_port" > "$portfile"; fi
}


# The child reports its bound port; poll briefly rather than sleeping a
# fixed wait, so startup is fast when it binds immediately.
wait_for_port() {
    local tries=0
    while [ ! -s "$portfile" ] || ! grep -qE '^[0-9]+$' "$portfile" 2>/dev/null; do
        sleep 0.2
        tries=$((tries + 1))
        if ! kill -0 "$(cat "$pidfile")" 2>/dev/null; then
            printf '%s: server died during startup; log tail:\n' "${0##*/}" >&2
            tail -5 "$HOME_DIR/server.log" >&2 || true
            exit 70
        fi
        [ "$tries" -lt 100 ] || { printf '%s: server did not report its port\n' "${0##*/}" >&2; exit 70; }
    done
}

do_start() {
    if is_running; then
        printf 'chat-server already running on port %s (pid %s)\n' \
            "$(cat "$portfile")" "$(cat "$pidfile")"
        exit 0
    fi
    mkdir -p "$CHAN_DIR" "$RUN_DIR"
    local runtime
    runtime="$(pick_runtime)" || {
        if [ -n "$want_runtime" ]; then
            printf '%s: runtime %s is not installed\n' "${0##*/}" "$want_runtime" >&2
            exit 66
        fi
        printf '%s: no chat server runtime found; need one of python3, node, perl, or socat\n' "${0##*/}" >&2
        printf 'client helpers still work against existing logs without a server\n' >&2
        exit 69
    }
    launch_runtime

    wait_for_port

    [ "$announce" = true ] && start_announce_beacon

    printf 'chat-server up: pid %s, port %s, runtime %s, home %s\n' \
        "$(cat "$pidfile")" "$(cat "$portfile")" "$runtime" "$HOME_DIR"
}

do_stop() {
    if is_running; then
        local pid
        pid="$(cat "$pidfile")"
        kill "$pid" 2>/dev/null || true
        local tries=0
        while kill -0 "$pid" 2>/dev/null && [ "$tries" -lt 50 ]; do sleep 0.1; tries=$((tries + 1)); done
        if [ -f "$HOME_DIR/announce.pid" ]; then
            kill "$(cat "$HOME_DIR/announce.pid")" 2>/dev/null || true
            rm -f "$HOME_DIR/announce.pid"
        fi
        rm -f "$pidfile" "$portfile"
        printf 'chat-server stopped\n'
    else
        if [ -f "$HOME_DIR/announce.pid" ]; then
            kill "$(cat "$HOME_DIR/announce.pid")" 2>/dev/null || true
            rm -f "$HOME_DIR/announce.pid"
        fi
        rm -f "$pidfile" "$portfile"
        printf 'chat-server not running\n'
    fi
}

do_status() {
    if is_running; then
        printf 'running: pid %s, port %s, bind %s\n' "$(cat "$pidfile")" \
            "$(cat "$portfile" 2>/dev/null || echo '?')" \
            "$(cat "$HOME_DIR/server.bind" 2>/dev/null || echo '?')"
        exit 0
    fi
    printf 'not running\n'
    exit 1
}

case "$cmd" in
    start) do_start ;;
    stop) do_stop ;;
    status) do_status ;;
esac
