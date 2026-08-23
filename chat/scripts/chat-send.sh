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
case "$nick" in *[!A-Za-z0-9_-]*) printf '%s: invalid nick\n' "${0##*/}" >&2; exit 64 ;; esac
[ -n "$text" ] || { printf '%s: empty message\n' "${0##*/}" >&2; exit 64; }
[ -z "$port" ] && [ -n "$host" ] && port=7717

if [ -n "$host" ]; then
    exec 3<> "/dev/tcp/$host/$port"
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
line="$(printf 'MSG %s %d %d %s :%s' "$chan" "$id" "$ts" "$nick" "$(printf '%s' "$text" | tr '\n\r' '  ')")"
printf '%s\n' "$line" >> "$log"
rmdir "$lock" 2>/dev/null || true
trap - EXIT
printf '%s\n' "$line"
