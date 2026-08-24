#!/usr/bin/env bash
# MODE: PROD
# chat-register.sh - create a chat channel so it exists before anyone sends.
# Idempotent: an existing channel answers OK, not 73.
#
# Usage:
#   chat-register.sh #chan [--home D]
#   chat-register.sh --help

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} #chan [--home D]

Creates channels/<chan>.log under AI_CHAT_HOME (default ~/.ai-chat) with
a 'system :channel registered' seed line as id 1.
USAGE
    exit "$rc"
}

chan="" home="${AI_CHAT_HOME:-$HOME/.ai-chat}"
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --home) [ "$#" -ge 2 ] || usage; home="$2"; shift 2 ;;
        -*) usage ;;
        *) [ -z "$chan" ] || usage; chan="$1"; shift ;;
    esac
done
[ -n "$chan" ] || usage
case "$chan" in '#'[a-z0-9_-]*) ;; *) printf '%s: channel must be #lowercase\n' "${0##*/}" >&2; exit 64 ;; esac

mkdir -p "$home/channels"
log="$home/channels/$chan.log"
if [ -f "$log" ]; then
    printf 'OK %s already exists: %s\n' "$chan" "$log"
    exit 0
fi
lock="$home/channels/$chan.lock"
tries=0
until mkdir "$lock" 2>/dev/null; do
    tries=$((tries + 1)); [ "$tries" -lt 100 ] || { printf '%s: lock timeout\n' "${0##*/}" >&2; exit 70; }
    sleep 0.05
done
if [ ! -f "$log" ]; then
    ts="$(date -u +%s)"
    printf 'MSG %s 1 %s system :channel registered\n' "$chan" "$ts" >> "$log"
fi
rmdir "$lock"
printf 'Registered %s: %s\n' "$chan" "$log"
