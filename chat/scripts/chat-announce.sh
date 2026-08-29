#!/usr/bin/env bash
# MODE: PROD
# chat-announce.sh — broadcast this chat server's existence on the LAN (T65).
#
# Sends a small JSON beacon over UDP every --interval seconds so agents can
# run chat-discover.sh instead of knowing an ip:port. The payload names the
# server for humans and machines: {"proto":"ai-chat/1","name":...,"port":...,
# "started":...}. Started by chat-server.sh --announce [name], or by hand.
#
# Usage:
#   chat-announce.sh --port N [--name NAME] [--interval S]
#                    [--bcast ADDR] [--beacon-port N] [--home D]
#   chat-announce.sh --help
#
# The beacon is UDP: lossy by design, repeated by interval, never guaranteed.
# Privacy: the default broadcast is the subnet's; --bcast 127.0.0.1 keeps it
# loopback-only for a machine-local setup.

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} --port N [--name NAME] [--interval S] [--bcast ADDR]
                 [--beacon-port N] [--home D]
       ${0##*/} --help
USAGE
    exit "$rc"
}

port="" name="" interval=2 bcast="255.255.255.255" beacon_port=7780
home="${AI_CHAT_HOME:-$HOME/.ai-chat}"

while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --port) [ "$#" -ge 2 ] || usage; port="$2"; shift 2 ;;
        --name) [ "$#" -ge 2 ] || usage; name="$2"; shift 2 ;;
        --interval) [ "$#" -ge 2 ] || usage; interval="$2"; shift 2 ;;
        --bcast) [ "$#" -ge 2 ] || usage; bcast="$2"; shift 2 ;;
        --beacon-port) [ "$#" -ge 2 ] || usage; beacon_port="$2"; shift 2 ;;
        --home) [ "$#" -ge 2 ] || usage; home="$2"; shift 2 ;;
        *) usage ;;
    esac
done

case "$port" in ''|*[!0-9]*) printf '%s: --port N is required\n' "${0##*/}" >&2; exit 64 ;; esac
case "$interval" in ''|*[!0-9]*) printf '%s: --interval needs a number\n' "${0##*/}" >&2; exit 64 ;; esac
case "$beacon_port" in ''|*[!0-9]*) printf '%s: --beacon-port needs a number\n' "${0##*/}" >&2; exit 64 ;; esac

# A human-identifiable, machine-stable name: explicit, or hostname + home
# basename, so two servers on one host differ and one server survives reboots.
if [ -z "$name" ]; then
    host_part="$(hostname 2>/dev/null || echo localhost)"
    home_part="$(basename "$home" | tr -c 'a-zA-Z0-9-' '-')"
    name="ai-chat/${host_part}-${home_part}"
fi
started="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
beacon="$(printf '{"proto":"ai-chat/1","name":"%s","port":%s,"started":"%s"}' \
    "$(printf '%s' "$name" | sed 's/"/\\"/g')" "$port" "$started")"

printf '%s: announcing "%s" on %s:%s every %ss\n' "${0##*/}" "$name" "$bcast" "$beacon_port" "$interval" >&2

# bash /dev/udp cannot send to a broadcast address (SO_BROADCAST is never
# set; the kernel answers EACCES), so python3 sends when present and
# /dev/udp stays as the unicast-only fallback (loopback or a unicast target).
if command -v python3 >/dev/null 2>&1; then
    exec python3 - "$bcast" "$beacon_port" "$interval" "$beacon" <<'PYSEND'
import socket, sys, time
bcast, port, interval, beacon = sys.argv[1], int(sys.argv[2]), float(sys.argv[3]), sys.argv[4]
s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
s.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
while True:
    try:
        s.sendto(beacon.encode(), (bcast, port))
    except OSError:
        pass  # no route yet: the interval retries
    time.sleep(interval)
PYSEND
fi

while :; do
    printf '%s' "$beacon" > "/dev/udp/$bcast/$beacon_port" 2>/dev/null || true
    sleep "$interval"
done
