#!/usr/bin/env bash
# MODE: PROD
# bash-handler.sh - per-connection handler for the socat tier of chat-server.
# socat TCP-LISTEN,...,fork EXECs this with the socket on stdin/stdout, so
# connections are isolated processes and live JOIN push is impossible: JOIN is
# answered as poll mode and clients use FETCH/chat-tail.sh. Every other verb
# behaves like the other runtimes.

set -euo pipefail
export LC_ALL=C

HOME_DIR="${AI_CHAT_HOME:?AI_CHAT_HOME must be set}"
CHAN_DIR="$HOME_DIR/channels"
mkdir -p "$CHAN_DIR"
LOCK_TRIES=200

valid_chan() {
    local rest
    case "$1" in
        '#'[a-z0-9_-]*)
            rest="${1#\#}"
            [ -n "$rest" ] && [ "${#rest}" -le 32 ] || return 1
            case "$rest" in *[!a-z0-9_-]*) return 1 ;; esac
            return 0 ;;
        *) return 1 ;;
    esac
}
valid_nick() { case "$1" in *[!A-Za-z0-9_-]*|"") return 1 ;; *) return 0 ;; esac; }
chan_path() { printf '%s/%s.log' "$CHAN_DIR" "$1"; }
bail() { printf 'ERR %s\n' "$1"; exit 70; }

last_id() {
    awk '$1 == "MSG" && $3 + 0 > last { last = $3 + 0 } END { print last + 0 }' \
        "$(chan_path "$1")" 2>/dev/null || printf '0'
}

# with_lock CHAN CMD... — run CMD holding the channel's mkdir lock.
with_lock() {
    local chan="$1" tries=0 rc=0
    shift
    until mkdir "$(chan_path "$chan").lock" 2>/dev/null; do
        [ -f "$(chan_path "$chan").lock" ] && bail "corrupt lock"
        tries=$((tries + 1))
        [ "$tries" -lt "$LOCK_TRIES" ] || bail "lock timeout"
        sleep 0.05
    done
    # B52: set -e would exit at a failing "$@" and leave the lock directory
    # behind, wedging every later connection on that channel; capture instead.
    rc=0
    "$@" || rc=$?
    rmdir "$(chan_path "$chan").lock"
    return "$rc"
}

append_msg() { # CHAN NICK TEXT
    local id ts line
    id=$(( $(last_id "$1") + 1 ))
    ts="$(date -u +%s)"
    line="$(printf 'MSG %s %d %d %s :%s\n' "$1" "$id" "$ts" "$2" "$3")"
    printf '%s\n' "$line" >> "$(chan_path "$1")"
    printf '%s' "$line"
}

nick="anon-$$"
reply() { printf '%s\n' "$1"; }

handle() {
    local verb arg chan rest since stored text
    verb="${line%% *}"
    arg=""
    case "$line" in *" "*) arg="${line#* }" ;; esac
    case "$verb" in
        NICK)
            valid_nick "$arg" || { reply "ERR invalid nick"; return; }
            nick="$arg"; reply "OK nick $arg" ;;
        REGISTER)
            valid_chan "$arg" || { reply "ERR invalid channel"; return; }
            with_lock "$arg" touch "$(chan_path "$arg")"
            reply "OK register $arg" ;;
        JOIN)
            valid_chan "$arg" || { reply "ERR invalid channel"; return; }
            reply "OK join $arg (poll mode)" ;;
        LEAVE)
            reply "OK leave $arg" ;;
        PRIVMSG)
            chan="${arg%% *}"
            rest=""
            case "$arg" in *" "*) rest="${arg#* }" ;; esac
            case "$rest" in ':'*) text="${rest#:}" ;; *) text="$rest" ;; esac
            valid_chan "$chan" || { reply "ERR invalid channel"; return; }
            [ -n "$text" ] || { reply "ERR usage: PRIVMSG #chan :text"; return; }
            text="$(printf '%s' "$text" | tr '\n\r' '  ')"
            stored="$(with_lock "$chan" append_msg "$chan" "$nick" "$text")"
            reply "$stored"
            ;;
        FETCH)
            chan="${arg%% *}"; since="${arg##* }"
            valid_chan "$chan" || { reply "ERR invalid channel"; return; }
            case "$since" in ''|*[!0-9]*) reply "ERR usage: FETCH #chan <since-id>"; return ;; esac
            awk -v s="$since" '$1 == "MSG" && ($3 + 0) > s' "$(chan_path "$chan")" 2>/dev/null || true
            reply "OK fetch end" ;;
        PING) reply "PONG" ;;
        QUIT) reply "OK bye"; exit 0 ;;
        *) reply "ERR unknown verb $verb" ;;
    esac
}

line=""
while IFS= read -r line; do
    line="${line%$'\r'}"
    [ -n "$line" ] || continue
    handle
done
