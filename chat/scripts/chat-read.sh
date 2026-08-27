#!/usr/bin/env bash
# MODE: PROD
# chat-read.sh - read stored messages from a channel: a delta since an id,
# the last N, or everything. Reads the local log unless --host names a server.
#
# Usage:
#   chat-read.sh #chan [--since N | --last N | --all] [--host H] [--port N] [--home D]
#   chat-read.sh #chan 41              # shorthand for --since 41
#   chat-read.sh --help

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} #chan [--since N | --last N | --all] [--host H] [--port N] [--home D]

Prints matching MSG lines in id order. --since wins over --last; bare number
as second argument means --since. Default is --all.
USAGE
    exit "$rc"
}

HOME_DIR="${AI_CHAT_HOME:-$HOME/.ai-chat}"
chan="" since="" last="" all="" host="" port=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --since|--last) [ "$#" -ge 2 ] || usage; if [ "$1" = --since ]; then since="$2"; else last="$2"; fi; shift 2 ;;
        --all) all=1; shift ;;
        --host) [ "$#" -ge 2 ] || usage; host="$2"; shift 2 ;;
        --port) [ "$#" -ge 2 ] || usage; port="$2"; shift 2 ;;
        --home) [ "$#" -ge 2 ] || usage; HOME_DIR="$2"; shift 2 ;;
        -*) usage ;;
        *) if [ -z "$chan" ]; then chan="$1"; elif [ -z "$since" ] && case "$1" in ''|*[!0-9]*) false ;; *) true ;; esac; then since="$1"; else usage; fi; shift ;;
    esac
done
[ -n "$chan" ] || usage
case "$chan" in '#'[a-z0-9_-]*) ;; *) printf '%s: channel must be #lowercase\n' "${0##*/}" >&2; exit 64 ;; esac

    # B58: every server tier caps the name at 32 chars after the #; clients
    # must refuse the same, or a channel is writable locally and unreachable
    # over the socket.
    [ "${#chan}" -le 33 ] || { printf '%s: channel name too long (max 32 after #): %s\n' "${0##*/}" "$chan" >&2; exit 64; }
[ -z "$port" ] && [ -n "$host" ] && port=7717

if [ -n "$host" ]; then
    exec 3<> "/dev/tcp/$host/$port"
    printf 'FETCH %s %s\nQUIT\n' "$chan" "${since:-0}" >&3
    while IFS= read -r line <&3; do
        case "$line" in
            MSG\ *) printf '%s\n' "$line" ;;
            OK\ fetch\ end*) break ;;
            ERR*) printf '%s: %s\n' "${0##*/}" "$line" >&2; exit 70 ;;
        esac
    done
    exec 3>&- 3<&-
    exit 0
fi

log="$HOME_DIR/channels/$chan.log"
[ -f "$log" ] || { printf '%s: no log for %s under %s\n' "${0##*/}" "$chan" "$HOME_DIR" >&2; exit 66; }

if [ -n "$since" ]; then
    awk -v s="$since" '$1 == "MSG" && ($3 + 0) > s' "$log"
elif [ -n "$last" ]; then
    case "$last" in ''|*[!0-9]*) usage ;; esac
    tail -n "$last" "$log"
else
    cat "$log"
fi
