#!/usr/bin/env bash
# MODE: PROD
# chat-tail.sh - the constant-output stream (default display IRC-style
# [HH:MM] <nick> text; --raw for stored MSG lines): print a channel's backlog since
# an id, then keep printing new messages as they land, until killed. This is
# how an agent "joins" a channel from a subshell.
#
# Local mode polls the log (the server need not be running); --host mode uses
# a socket JOIN and reads pushed lines instead.
#
# Usage:
#   chat-tail.sh #chan [since-id] [--interval N] [--host H] [--port N] [--home D]
#   chat-tail.sh --help

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} #chan [since-id] [--interval N] [--raw] [--host H] [--port N] [--home D]

since-id defaults to 0 on a socket (full replay) and to "end of log" locally
(pass 0 to replay everything). Interval is the local poll seconds (default 1,
integer: bash 3.2 has no fractional read timeouts).
USAGE
    exit "$rc"
}

HOME_DIR="${AI_CHAT_HOME:-$HOME/.ai-chat}"
chan="" raw=false since="" interval=1 host="" port=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --raw) raw=true; shift ;;
        --interval) [ "$#" -ge 2 ] || usage; interval="$2"; shift 2 ;;
        --host) [ "$#" -ge 2 ] || usage; host="$2"; shift 2 ;;
        --port) [ "$#" -ge 2 ] || usage; port="$2"; shift 2 ;;
        --home) [ "$#" -ge 2 ] || usage; HOME_DIR="$2"; shift 2 ;;
        -*) usage ;;
        *) if [ -z "$chan" ]; then chan="$1"
           elif [ -z "$since" ] && case "$1" in ''|*[!0-9]*) false ;; *) true ;; esac; then since="$1"
           else usage; fi
           shift ;;
    esac
done
[ -n "$chan" ] || usage
case "$chan" in '#'[a-z0-9_-]*) ;; *) printf '%s: channel must be #lowercase\n' "${0##*/}" >&2; exit 64 ;; esac

    # B58: every server tier caps the name at 32 chars after the #; clients
    # must refuse the same, or a channel is writable locally and unreachable
    # over the socket.
    [ "${#chan}" -le 33 ] || { printf '%s: channel name too long (max 32 after #): %s\n' "${0##*/}" "$chan" >&2; exit 64; }
case "$interval" in ''|*[!0-9]*|0*) interval=1 ;; esac
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

if [ -n "$host" ]; then
    exec 3<> "/dev/tcp/$host/$port"
    # B66: the since-id rides the JOIN so the server replays the backlog
    # from the requested point (0 = full history) before live traffic.
    printf 'NICK %s\nJOIN %s %s\n' "${CHAT_NICK:-${USER:-agent}-tail}" "$chan" "${since:-0}" >&3
    while IFS= read -r line <&3; do
        case "$line" in
            MSG\ *) if [ "$raw" = true ]; then
                        printf '%s\n' "$line"
                    else
                        printf '%s\n' "$line" | awk '$1 == "MSG" {
                            ts = $4; h = int(ts / 3600) % 24; mi = int(ts / 60) % 60
                            sep = index($0, " :")
                            printf "[%02d:%02d] <%s> %s\n", h, mi, $5, substr($0, sep + 2)
                        }'
                    fi ;;
            OK\ join*) : ;;
            ERR*) printf '%s: %s\n' "${0##*/}" "$line" >&2; exit 70 ;;
        esac
    done
    exit 0
fi

log="$HOME_DIR/channels/$chan.log"
[ -f "$log" ] || { printf '%s: no log for %s under %s\n' "${0##*/}" "$chan" "$HOME_DIR" >&2; exit 66; }

if [ -z "$since" ]; then
    since="$(awk '$1 == "MSG" && $3 + 0 > last { last = $3 + 0 } END { print last + 0 }' "$log")"
fi

# Stable temp files for the poll loop; removed when this tail dies.
tmp="$(mktemp "${TMPDIR:-/tmp}/chat-tail.XXXXXX")"
out="$tmp.found"
trap 'rm -f "$tmp" "$out"' EXIT
printf '%s\n' "$since" > "$tmp"
: > "$out"

while :; do
    if [ -f "$log" ]; then
        awk -v s="$(cat "$tmp")" '$1 == "MSG" && ($3 + 0) > s' "$log" > "$out" || true
        if [ -s "$out" ]; then
            if [ "$raw" = true ]; then
                cat "$out"
            else
                cat "$out" | awk '$1 == "MSG" {
                    ts = $4; h = int(ts / 3600) % 24; mi = int(ts / 60) % 60
                    sep = index($0, " :")
                    printf "[%02d:%02d] <%s> %s\n", h, mi, $5, substr($0, sep + 2)
                }'
            fi
            lastid="$(awk '$1 == "MSG" && $3 + 0 > m { m = $3 + 0 } END { print m + 0 }' "$out")"
            printf '%s\n' "$lastid" > "$tmp"
        fi
        : > "$out"
    fi
    sleep "$interval"
done
