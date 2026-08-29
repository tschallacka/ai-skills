#!/usr/bin/env bash
# MODE: PROD
# chat-discover.sh — find chat servers announcing on the LAN (T65).
#
# Listens one beacon period, dedupes by name+port, and prints a numbered list
# a human can choose from. --json prints the array for an agent instead.
#
# The joining agent MUST ask its driving human before connecting to any
# network host (chat/SKILL.md, "Joining a server"): present this list plus
# "start your own local server" as options, and in the same conversation ask
# whether the human wants to set the agent's nickname or let the agent pick.
#
# Usage:
#   chat-discover.sh [--wait S] [--beacon-port N] [--bcast ADDR] [--json]
#   chat-discover.sh --help

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--wait S] [--beacon-port N] [--bcast ADDR] [--json]
       ${0##*/} --help
USAGE
    exit "$rc"
}

wait_s=6 beacon_port=7780 bcast="255.255.255.255" json=false
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --wait) [ "$#" -ge 2 ] || usage; wait_s="$2"; shift 2 ;;
        --beacon-port) [ "$#" -ge 2 ] || usage; beacon_port="$2"; shift 2 ;;
        --bcast) [ "$#" -ge 2 ] || usage; bcast="$2"; shift 2 ;;
        --json) json=true; shift ;;
        *) usage ;;
    esac
done
case "$wait_s" in ''|*[!0-9]*) printf '%s: --wait needs a number\n' "${0##*/}" >&2; exit 64 ;; esac
case "$beacon_port" in ''|*[!0-9]*) printf '%s: --beacon-port needs a number\n' "${0##*/}" >&2; exit 64 ;; esac

work="$(mktemp -d "${TMPDIR:-/tmp}/chat-discover.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# bash /dev/udp can send but not receive, so the reader needs a tier; socat
# is the natural UDP reader, python3 otherwise, and neither is UNCONFIGURED
# (a skip, not a failure — a host without either still has the local server).
reader=""
if command -v socat >/dev/null 2>&1; then
    reader=socat
elif command -v python3 >/dev/null 2>&1; then
    reader=python3
else
    printf 'chat-discover: UNCONFIGURED (need socat or python3 to read UDP)\n' >&2
    exit 0
fi

case "$reader" in
    socat)
        timeout "$wait_s" socat -u "UDP4-RECVFROM:$beacon_port,reuseaddr" - \
            >> "$work/raw" 2>/dev/null || true
        ;;
    python3)
        timeout "$wait_s" python3 - "$beacon_port" "$wait_s" >> "$work/raw" <<'PYEOF' 2>/dev/null || true
import socket, sys, time
port = int(sys.argv[1])
deadline = time.time() + float(sys.argv[2])
s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
try:
    s.bind(("", port))
except OSError:
    sys.exit(0)  # someone else is already listening; not our beacon to read
s.settimeout(0.5)
while time.time() < deadline:
    try:
        data, _addr = s.recvfrom(2048)
        sys.stdout.write(data.decode(errors="replace") + "\n")
        sys.stdout.flush()
    except socket.timeout:
        continue
PYEOF
        ;;
esac

# The beacon repeats every interval and several servers may answer; the list
# shows each name+port once.
pairs="$(grep '^{"proto"' "$work/raw" 2>/dev/null | sort -u || true)"

if [ -z "$pairs" ]; then
    if [ "$json" = true ]; then
        printf '[]\n'
    else
        printf 'chat-discover: no servers found within %ss (beacon port %s)\n' \
            "$wait_s" "$beacon_port" >&2
    fi
    exit 0
fi

if [ "$json" = true ]; then
    printf '%s\n' "$pairs" | awk '
        BEGIN { printf "[" }
        {
            if (n++) printf ","
            printf "%s", $0
        }
        END { if (n) printf "\n"; printf "]\n" }
    '
else
    printf '%s\n' "$pairs" | awk '
        {
            if (match($0, /"name":"[^"]*"/)) name = substr($0, RSTART + 8, RLENGTH - 9)
            if (match($0, /"port":[0-9]+/)) port = substr($0, RSTART + 7, RLENGTH - 7)
            key = name "|" port
            if (!(key in seen)) { seen[key] = 1; printf "%d. %s  (port %s)\n", ++n, name, port }
        }
    '
fi
