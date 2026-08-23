#!/usr/bin/env bash
# MODE: PROD
# chat-server.sh - start/stop/status the chat socket server, choosing the
# first available runtime: python3, node, perl, or socat driving the bash
# handler (poll mode). State lives under $AI_CHAT_HOME (default ~/.ai-chat).
#
# Usage:
#   chat-server.sh start [--runtime R] [--port N] [--home D]
#   chat-server.sh stop|status [--home D]
#   chat-server.sh --help
#
# start waits until the chosen port is written to <home>/server.port and then
# prints it. Exit codes: 64 bad invocation, 66 runtime missing, 69 no runtime
# at all, 70 internal.

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} start [--runtime python3|node|perl|socat] [--port N] [--home D]
       ${0##*/} status [--home D]
       ${0##*/} stop   [--home D]

The runtime chain falls back through what is installed; --runtime pins one.
Without any of them the server cannot open a socket (exit 69) while every
client helper keeps working against existing logs.
USAGE
    exit "$rc"
}

HOME_DIR="${AI_CHAT_HOME:-$HOME/.ai-chat}"
cmd="" want_runtime="" want_port=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        start|stop|status) [ -z "$cmd" ] || usage; cmd="$1"; shift ;;
        --runtime) [ "$#" -ge 2 ] || usage; want_runtime="$2"; shift 2 ;;
        --port) [ "$#" -ge 2 ] || usage; want_port="$2"; shift 2 ;;
        --home) [ "$#" -ge 2 ] || usage; HOME_DIR="$2"; shift 2 ;;
        *) usage ;;
    esac
done
[ -n "$cmd" ] || usage
case "$want_port" in ''|*[!0-9]*) want_port="" ;; esac

CHAN_DIR="$HOME_DIR/channels"
RUN_DIR="$HOME_DIR/run"
pidfile="$HOME_DIR/server.pid"
portfile="$HOME_DIR/server.port"

is_running() {
    [ -f "$pidfile" ] || return 1
    kill -0 "$(cat "$pidfile")" 2>/dev/null
}

pick_runtime() {
    local order="${want_runtime:-python3 node perl socat}"
    for r in $order; do
        command -v "$r" >/dev/null 2>&1 && { printf '%s\n' "$r"; return 0; }
    done
    return 1
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
        printf '%s: no chat server runtime found; need one of python3, node, perl, or socat\n' "${0##*/}" >&2
        printf 'client helpers still work against existing logs without a server\n' >&2
        exit 69
    }
    # Keep the real extension: node dlopens *.node as a native addon.
    local script="$RUN_DIR/placeholder"
    local src
    case "$runtime" in
        python3) src=server.py ;;
        node) src=server.js ;;
        perl) src=server.pl ;;
    esac
    case "$runtime" in
        python3|node|perl)
            script="$RUN_DIR/$src"
            cp "$(dirname "${BASH_SOURCE[0]}")/../runtime/$src" "$script" ;;
        socat)
            command -v socat >/dev/null 2>&1 || {
                printf '%s: socat selected but not installed\n' "${0##*/}" >&2
                exit 66
            }
            case "$want_port" in ''|0)
                printf '%s: the socat runtime needs an explicit --port N\n' "${0##*/}" >&2
                exit 64 ;;
            esac
            script=""
            ;;
    esac

    : > "$HOME_DIR/server.log"
    if [ "$runtime" = socat ]; then
        handler="$(dirname "${BASH_SOURCE[0]}")/../runtime/bash-handler.sh"
        AI_CHAT_HOME="$HOME_DIR" nohup socat "TCP-LISTEN:${want_port:-0},fork,reuseaddr,bind=127.0.0.1" \
            "EXEC:$handler" >> "$HOME_DIR/server.log" 2>&1 &
    else
        AI_CHAT_HOME="$HOME_DIR" nohup ${runtime} ${script:+$script} ${want_port:+$want_port} \
            >> "$HOME_DIR/server.log" 2>&1 &
    fi
    echo $! > "$pidfile"
    # socat names its port on the command line; the others report their bound
    # port themselves once listening.
    if [ "$runtime" = socat ]; then printf '%s\n' "$want_port" > "$portfile"; fi

    # The child reports its bound port; poll briefly rather than sleeping a
    # fixed wait, so startup is fast when it binds immediately.
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

    # socat never writes the port file itself: ask the socket what we opened.
    if [ "$runtime" = socat ]; then
        local p="${want_port:-}"
        if [ -z "$p" ]; then
            p="$(awk '/NCAT|socat.*listening on/{print $NF}' "$HOME_DIR/server.log" 2>/dev/null | head -1)"
            case "$p" in ''|*[!0-9]*) p="?" ;; esac
        fi
        printf '%s\n' "$p" > "$portfile"
    fi
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
        rm -f "$pidfile" "$portfile"
        printf 'chat-server stopped\n'
    else
        rm -f "$pidfile" "$portfile"
        printf 'chat-server not running\n'
    fi
}

do_status() {
    if is_running; then
        printf 'running: pid %s, port %s\n' "$(cat "$pidfile")" "$(cat "$portfile" 2>/dev/null || echo '?')"
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
