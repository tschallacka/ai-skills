#!/usr/bin/env bash
# MODE: PROD
# chat-send.sh - append one message to a channel. Without --host it appends
# directly under the channel lock (the server is not needed and must not be
# running as sole writer); with --host it speaks PRIVMSG over the socket so a
# remote server stores it.
#
# Usage:
#   chat-send.sh #chan "text" [-n NICK] [--host H] [--port N] [--home D]
#   chat-send.sh --help

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} #chan "text" [-n NICK] [--host H] [--port N] [--home D]

Prints the stored line on success:  MSG #chan <id> <ts> <nick> :text
NICK defaults to \${CHAT_NICK:-\${USER:-agent}}.
USAGE
    exit "$rc"
}

HOME_DIR="${AI_CHAT_HOME:-$HOME/.ai-chat}"
nick="${CHAT_NICK:-${USER:-agent}}"
host="" port="" chan="" text=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        -n) [ "$#" -ge 2 ] || usage; nick="$2"; shift 2 ;;
        --host) [ "$#" -ge 2 ] || usage; host="$2"; shift 2 ;;
        --port) [ "$#" -ge 2 ] || usage; port="$2"; shift 2 ;;
        --home) [ "$#" -ge 2 ] || usage; HOME_DIR="$2"; shift 2 ;;
        *) if [ -z "$chan" ]; then chan="$1"; elif [ -z "$text" ]; then text="$1"; else usage; fi; shift ;;
    esac
done
[ -n "$chan" ] || usage
case "$chan" in '#'[a-z0-9_-]*) ;; *) printf '%s: channel must be #lowercase\n' "${0##*/}" >&2; exit 64 ;; esac

    # B58: every server tier caps the name at 32 chars after the #; clients
    # must refuse the same, or a channel is writable locally and unreachable
    # over the socket.
    [ "${#chan}" -le 33 ] || { printf '%s: channel name too long (max 32 after #): %s\n' "${0##*/}" "$chan" >&2; exit 64; }
case "$nick" in *[!A-Za-z0-9_-]*) printf '%s: invalid nick\n' "${0##*/}" >&2; exit 64 ;; esac
[ -n "$text" ] || { printf '%s: empty message\n' "${0##*/}" >&2; exit 64; }

# B74: one message is one line, on the wire and in the log, so the body must
# carry no line break on EITHER path. The socket path used to interpolate the
# raw body into 'NICK %s\nPRIVMSG %s :%s\n', which made every newline in a
# relayed diff, file excerpt or quotation a command boundary — a body could
# forge a NICK and post as anyone. Sanitising here rather than refusing keeps
# the socket path doing exactly what the direct append always did, and keeps
# relaying multi-line text (B75's surface) working instead of erroring.
text="$(printf '%s' "$text" | tr '\n\r' '  ')"

[ -z "$port" ] && [ -n "$host" ] && port=7717

# The transport, in precedence order: an explicit --host/--port wins, then the
# recorded config, then the built-in default. Sourced rather than reimplemented
# three times, because three copies of a precedence rule is three chances to
# disagree with the binary about where the bus is.
# shellcheck source=chat/scripts/lib-config.sh
. "$(dirname "${BASH_SOURCE[0]}")/lib-config.sh"
if [ -z "$host" ]; then
    chat_config_target "$HOME_DIR" "${0##*/}"
    host="$CHAT_USE_HOST"
    [ -n "$port" ] || port="$CHAT_USE_PORT"
else
    # --host with no --port takes the recorded port, so a bus moved off the
    # default does not have to be spelled out twice.
    if [ -z "$port" ]; then
        chat_config_load "$HOME_DIR"
        case "$CHAT_TRANSPORT" in
            tcp) port="$CHAT_CFG_PORT" ;;
            *) port="$CHAT_DEFAULT_PORT" ;;
        esac
    fi
fi

# B65: a local send with a server running must not bypass it — a direct log
# append is invisible to every socket subscriber. When no --host was given
# but this home has a live server, route through its socket and fall back to
# the direct append only when the socket cannot be reached.
if [ -z "$host" ] && [ -f "$HOME_DIR/server.pid" ] && [ -f "$HOME_DIR/server.port" ]; then
    spid="$(cat "$HOME_DIR/server.pid" 2>/dev/null || true)"
    sport="$(cat "$HOME_DIR/server.port" 2>/dev/null || true)"
    case "$sport" in ''|*[!0-9]*) sport="" ;; esac
    if [ -n "$spid" ] && [ -n "$sport" ] && kill -0 "$spid" 2>/dev/null         && (exec 3<> "/dev/tcp/127.0.0.1/$sport") 2>/dev/null; then
        host=127.0.0.1
        port="$sport"
        printf '%s: local server on port %s; sending through it\n' "${0##*/}" "$sport" >&2
    fi
fi

if [ -n "$host" ]; then
    if exec 3<> "/dev/tcp/$host/$port" 2>/dev/null; then
        printf 'NICK %s\nPRIVMSG %s :%s\nQUIT\n' "$nick" "$chan" "$text" >&3
        stored=""
        while IFS= read -r line <&3; do
            case "$line" in
                MSG\ *"$chan"\ *) stored="$line"; break ;;
                ERR*) printf '%s: %s\n' "${0##*/}" "$line" >&2; exit 70 ;;
                OK\ bye*) break ;;
            esac
        done
        exec 3>&- 3<&-
        [ -n "$stored" ] || { printf '%s: no acknowledgement from %s:%s\n' "${0##*/}" "$host" "$port" >&2; exit 70; }
        printf '%s\n' "$stored"
        exit 0
    fi
    printf '%s: server %s:%s is unavailable; using the log directly\n' \
        "${0##*/}" "$host" "$port" >&2
    exec 3>&- 3<&-
fi

mkdir -p "$HOME_DIR/channels"
log="$HOME_DIR/channels/$chan.log"
lock="$HOME_DIR/channels/$chan.lock"
tries=0
until mkdir "$lock" 2>/dev/null; do
    tries=$((tries + 1)); [ "$tries" -lt 200 ] || { printf '%s: lock timeout\n' "${0##*/}" >&2; exit 70; }
    sleep 0.05
done
trap 'rmdir "$lock" 2>/dev/null || true' EXIT
id=$(( $(awk '$1 == "MSG" && $3 + 0 > last { last = $3 + 0 } END { print last + 0 }' "$log" 2>/dev/null || echo 0) + 1 ))
ts="$(date -u +%s)"
line="$(printf 'MSG %s %d %d %s :%s' "$chan" "$id" "$ts" "$nick" "$text")"
printf '%s\n' "$line" >> "$log"
rmdir "$lock" 2>/dev/null || true
trap - EXIT
printf '%s\n' "$line"
