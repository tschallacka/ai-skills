#!/usr/bin/env bash
# Run a command with a memory limit and, where supported, a CPU quota.
# Linux uses a transient systemd --user cgroup scope. macOS has no reliable
# per-process RAM limit, so it uses nice and optionally cpulimit instead.
#
# Usage:
#   limited-run.sh <memory-max> <cpu-quota-percent> -- <command> [args...]
#
# Examples:
#   limited-run.sh 2G 400 -- <test-command> ...
#   limited-run.sh 6G 400 -- <browser-driver> ...
#
# MemoryMax accepts systemd byte suffixes (G, M, ...). CPUQuota is a percentage
# of a single core (400 = up to 4 cores' worth of CPU time).

set -euo pipefail

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
        echo "Warning: macOS cannot enforce the requested RAM limit ($mem)." >&2
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

case "$mem" in
    *G) memory_kb=$(( ${mem%G} * 1024 * 1024 )) ;;
    *M) memory_kb=$(( ${mem%M} * 1024 )) ;;
    *K) memory_kb=${mem%K} ;;
    *)
        echo "Unsupported memory limit: $mem (use K, M, or G suffix)" >&2
        exit 64
        ;;
esac

if ! ulimit -v "$memory_kb"; then
    echo "Could not apply a virtual-memory limit on $(uname -s)" >&2
    exit 69
fi

exec "$@"
