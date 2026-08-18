#!/usr/bin/env bash
# Run a command with a memory limit and, where supported, a CPU quota.
# Linux uses a transient systemd --user cgroup scope. macOS uses memlimit, a
# best-effort preventive cap, and degrades to nice/cpulimit without it.
#
# Usage:
#   limited-run.sh <memory-max> <cpu-quota-percent> -- <command> [args...]
#
# Examples:
#   limited-run.sh 2G 400 -- <test-command> ...
#   limited-run.sh 6G 400 -- <browser-driver> ...
#
# MemoryMax accepts systemd byte suffixes (G, M, K). CPUQuota is a percentage
# of a single core (400 = up to 4 cores' worth of CPU time).
#
# Exit codes: 64 bad usage, 69 no limit mechanism on this operating system.

set -euo pipefail

# KiB for the K/M/G suffixes MemoryMax takes. Digits are checked here because
# an unchecked ${mem%G} reaches shell arithmetic and dies as a syntax error
# instead of the usage refusal a caller can branch on.
memory_kib() {
    local value="$1" digits="" multiplier=""
    case "$value" in
        *G) digits="${value%G}"; multiplier=$(( 1024 * 1024 )) ;;
        *M) digits="${value%M}"; multiplier=1024 ;;
        *K) digits="${value%K}"; multiplier=1 ;;
        *) return 1 ;;
    esac
    case "$digits" in
        ''|*[!0-9]*) return 1 ;;
    esac
    printf '%s\n' "$(( digits * multiplier ))"
}

if [ "$#" -lt 4 ] || [ "$3" != "--" ]; then
    echo "Usage: $(basename "$0") <memory-max> <cpu-quota-percent> -- <command> [args...]" >&2
    exit 64
fi

mem="$1"
cpu="$2"
shift 3

case "$(uname -s)" in
    Linux)
        if command -v systemd-run >/dev/null 2>&1 \
            && command -v systemctl >/dev/null 2>&1 \
            && systemctl --user show-environment >/dev/null 2>&1; then
            exec systemd-run --user --scope --quiet \
                -p MemoryMax="$mem" \
                -p MemorySwapMax=0 \
                -p CPUQuota="${cpu}%" \
                -- "$@"
        fi
        ;;
    Darwin)
        # memlimit — MIT, Jelle Besseling (pingiun),
        # https://github.com/pingiun/memlimit — refuses an allocation past the
        # cap instead of killing. Best-effort; not a cgroup-equivalent.
        if command -v memlimit >/dev/null 2>&1; then
            if ! memory_kb="$(memory_kib "$mem")"; then
                echo "Unsupported memory limit: $mem (use K, M, or G suffix)" >&2
                exit 64
            fi
            memory_bytes=$(( memory_kb * 1024 ))
            # nice stays outside memlimit: /usr/bin/nice is SIP-protected, so
            # dyld would scrub DYLD_* and the cap with it. cpulimit goes inside,
            # where it is injectable and its child joins the capped tree.
            if command -v cpulimit >/dev/null 2>&1; then
                exec nice -n 10 memlimit "$memory_bytes" -- \
                    cpulimit --limit="$cpu" -- "$@"
            fi
            exec nice -n 10 memlimit "$memory_bytes" -- "$@"
        fi
        echo "Warning: the requested RAM limit ($mem) is not enforced." >&2
        if [ "$(uname -m)" = arm64 ]; then
            echo "Install memlimit: curl -LsSf https://github.com/pingiun/memlimit/releases/latest/download/memlimit-installer.sh | sh" >&2
        else
            echo "memlimit supports Apple Silicon only, so this Intel Mac has no memory-cap mechanism." >&2
        fi
        echo "Applying nice priority; use cpulimit for a best-effort CPU throttle." >&2
        if command -v cpulimit >/dev/null 2>&1; then
            exec cpulimit --limit="$cpu" -- nice -n 10 "$@"
        fi
        exec nice -n 10 "$@"
        ;;
    *)
        echo "Unsupported operating system: $(uname -s)" >&2
        exit 69
        ;;
esac

if ! memory_kb="$(memory_kib "$mem")"; then
    echo "Unsupported memory limit: $mem (use K, M, or G suffix)" >&2
    exit 64
fi

if ! ulimit -v "$memory_kb"; then
    echo "Could not apply a virtual-memory limit on $(uname -s)" >&2
    exit 69
fi

exec "$@"
