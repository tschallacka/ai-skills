#!/usr/bin/env bash
# MODE: PROD
# chat-watch.sh — poll a channel with a step-down cadence instead of a fixed
# sleep. Live socket push is the better tool when available (chat-tail); this
# is for agents that cannot hold a connection open.
#
# Cadence (T63): poll twice every 5s; no new message -> twice every 10s, then
# twice every 20s, +10s per round until 60s is the steady state. Any new
# message resets the cadence to 5s so a live conversation answers fast.
#
# Usage:
#   chat-watch.sh #chan [--since N] [--max-wait S] [--interval-cap S]
#                 [--host H] [--port N] [--home D]
#   chat-watch.sh --help
#
# Prints each new message as it arrives (MSG lines only). Exits 0 on timeout,
# nonzero on a hard error. --max-wall S bounds the whole run (default 0 =
# unbounded); each poll uses chat-read's FETCH since-id, so no message is
# missed and none is printed twice.

set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} #chan [--since N] [--max-wall S] [--interval-cap S]
       ${0##*/} #chan --host H --port N [--since N] [--max-wall S]
       ${0##*/} --help
USAGE
    exit "$rc"
}

chan="" since="" max_wall=0 cap=60 host="" port=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --since) [ "$#" -ge 2 ] || usage; since="$2"; shift 2 ;;
        --max-wall) [ "$#" -ge 2 ] || usage; max_wall="$2"; shift 2 ;;
        --interval-cap) [ "$#" -ge 2 ] || usage; cap="$2"; shift 2 ;;
        --host) [ "$#" -ge 2 ] || usage; host="$2"; shift 2 ;;
        --port) [ "$#" -ge 2 ] || usage; port="$2"; shift 2 ;;
        --home) [ "$#" -ge 2 ] || usage; export AI_CHAT_HOME="$2"; shift 2 ;;
        --) shift; break ;;
        -*) usage ;;
        *) [ -z "$chan" ] || usage; chan="$1"; shift ;;
    esac
done
case "$chan" in '#'[a-z0-9_-]*) ;; *) printf '%s: channel must be #lowercase\n' "${0##*/}" >&2; exit 64 ;; esac
[ "${#chan}" -le 33 ] || { printf '%s: channel name too long (max 32 after #): %s\n' "${0##*/}" "$chan" >&2; exit 64; }
for v in since max_wall cap port; do
    case "${!v:-}" in ''|*[!0-9]*) [ -z "${!v:-}" ] || { printf '%s: --%s needs a number\n' "${0##*/}" "$v" >&2; exit 64; } ;; esac
done
[ -n "$port" ] && [ -z "$host" ] && { printf '%s: --port needs --host\n' "${0##*/}" >&2; exit 64; }
[ -n "$host" ] && [ -z "$port" ] && port=7717

# B76: read_args must NOT carry --since. chat-read's parser is last-wins, so
# appending the ORIGINAL --since after the per-poll --since made every poll
# re-request the window the watcher started with — the same ids emitted again
# and again, on the socket path and the local one alike. The starting id lives
# in the cursor and nowhere else.
read_args=()
[ -n "$host" ] && read_args=(--host "$host" --port "$port")

# chat-read prints MSG lines; the cursor is the HIGHEST id printed so far,
# because both selections are strictly greater than since-id — the local awk
# is '($3 + 0) > s' and every server tier's FETCH is '> since'. A cursor set
# one past the id would silently skip the very next message.
cursor="${since:-0}"
poll() {
    local out id
    out="$("$script_dir/chat-read.sh" "$chan" --since "$cursor" "${read_args[@]+"${read_args[@]}"}" 2>/dev/null || true)"
    [ -n "$out" ] || return 1
    printf '%s\n' "$out"
    # advance the cursor to the highest id printed; take the max rather than
    # the last line, so an out-of-order log cannot rewind the cursor.
    id="$(printf '%s\n' "$out" | awk '$1 == "MSG" && $3 + 0 > c { c = $3 + 0 } END { print c + 0 }')"
    [ "$id" -gt "$cursor" ] && cursor="$id"
    return 0
}

# One flat loop, one sleep per iteration: the poll COUNT since the last
# activity decides the step, so the gaps are exactly twice-per-interval
# (5,5 -> 10,10 -> 20,20 -> ... capped at --interval-cap, default 60).
interval=5
quiet=0
stepped=0
started=$(date +%s)
while :; do
    if poll; then
        interval=5   # a message resets the cadence to fast
        quiet=0
        stepped=0
    else
        quiet=$((quiet + 1))
        if [ "$quiet" -ge 2 ]; then
            quiet=0
            if [ "$stepped" -eq 0 ]; then
                interval=10   # 5 -> 10 is the first step; the rest are +10
                stepped=1
            else
                interval=$(( interval + 10 ))
            fi
            [ "$interval" -gt "$cap" ] && interval="$cap"
        fi
    fi
    if [ "$max_wall" -gt 0 ]; then
        now=$(date +%s)
        [ $(( now - started )) -ge "$max_wall" ] && exit 0
    fi
    sleep "$interval"
done
