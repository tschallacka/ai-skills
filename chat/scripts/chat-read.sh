#!/usr/bin/env bash
# MODE: PROD
# chat-read.sh - read stored messages from a channel: a delta since an id,
# the last N, or everything. Reads the local log unless --host names a server.
#
# Usage:
#   chat-read.sh #chan [--since N | --last N | --all] [--mentions NICK]
#                 [--host H] [--port N] [--home D]
#   chat-read.sh #chan 41              # shorthand for --since 41
#   chat-read.sh --help

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} #chan [--since N | --last N | --all] [--mentions NICK]
                 [--pretty] [--host H] [--port N] [--home D]

Prints matching MSG lines in id order. --since wins over --last; bare number
as second argument means --since. Default is --all.

--mentions NICK keeps only lines whose text carries @NICK or @all/@everyone
(advisory metadata, not delivery: offline agents see mentions via history).
--pretty renders for humans, IRC-style: [HH:MM] <nick> text. The stored MSG
line is untouched — parsers keep their field positions.
USAGE
    exit "$rc"
}

HOME_DIR="${AI_CHAT_HOME:-$HOME/.ai-chat}"
chan="" mentions="" pretty=false since="" last="" all="" host="" port=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --since|--last) [ "$#" -ge 2 ] || usage; if [ "$1" = --since ]; then since="$2"; else last="$2"; fi; shift 2 ;;
        --mentions) [ "$#" -ge 2 ] || usage; mentions="$2"; shift 2 ;;
        --pretty) pretty=true; shift ;;
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

mention_ok() { # LINE — true when the line passes the --mentions filter
    [ -z "$mentions" ] && return 0
    # A mention is @nick delimited by space, punctuation or end-of-line, so
    # @oxen does not count as a mention of ox.
    [[ $1 == *@"$mentions"[' ,.:;!?'] || $1 == *@"$mentions" ]] && return 0
    [[ $1 == *@all* || $1 == *@everyone* ]] && return 0
    return 1
}

if [ -n "$host" ]; then
    exec 3<> "/dev/tcp/$host/$port"
    printf 'FETCH %s %s\nQUIT\n' "$chan" "${since:-0}" >&3
    while IFS= read -r line <&3; do
        case "$line" in
            MSG\ *) if mention_ok "$line"; then
                        if [ "$pretty" = true ]; then
                            printf '%s\n' "$line" | render
                        else
                            printf '%s\n' "$line"
                        fi
                    fi ;;
            OK\ fetch\ end*) break ;;
            ERR*) printf '%s: %s\n' "${0##*/}" "$line" >&2; exit 70 ;;
        esac
    done
    exec 3>&- 3<&-
    exit 0
fi

log="$HOME_DIR/channels/$chan.log"
[ -f "$log" ] || { printf '%s: no log for %s under %s\n' "${0##*/}" "$chan" "$HOME_DIR" >&2; exit 66; }

# --mentions filters every local mode, so its stage sits after selection.
select_lines() {
    if [ -n "$since" ]; then
        awk -v s="$since" '$1 == "MSG" && ($3 + 0) > s' "$log"
    elif [ -n "$last" ]; then
        case "$last" in ''|*[!0-9]*) usage ;; esac
        tail -n "$last" "$log"
    else
        cat "$log"
    fi
}
render() {
    if [ "$pretty" = true ]; then
        awk '$1 == "MSG" {
            ts = $4; h = int(ts / 3600) % 24; mi = int(ts / 60) % 60
            sep = index($0, " :")
            printf "[%02d:%02d] <%s> %s\n", h, mi, $5, substr($0, sep + 2)
        }'
    else
        cat
    fi
}
if [ -n "$mentions" ]; then
    select_lines | awk -v m="$mentions" '
        $1 != "MSG" { next }
        $0 ~ ("@" m "([ ,.:;!?]|$)") { print; next }
        /@all|@everyone/ { print }
    ' | render
else
    select_lines | render
fi
