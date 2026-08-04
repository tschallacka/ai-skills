#!/usr/bin/env bash
# Run a command with a memory limit and, where supported, a CPU quota.
# Linux uses a transient systemd --user cgroup scope. macOS uses the shell's
# virtual-memory limit because it has no portable built-in cgroup equivalent.
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
        # macOS has no portable built-in CPU quota. Continue with the memory
        # limit and keep the interface compatible with the Linux invocation.
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
