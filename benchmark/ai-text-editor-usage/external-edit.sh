#!/usr/bin/env bash
# MODE: DEV
# The "someone edits it under your nose" probe: waits for a MARKER FILE to
# appear in the workspace -- tax_rates.csv, task-prompt.md's own checkpoint
# for "the three task_source.py edits are done" -- rather than a fixed
# wall-clock delay, then mutates task_source.py directly -- outside either
# condition's own tools, simulating a teammate's own concurrent edit.
# Changes exactly one line: the TAX_RATE constant, which `apply_tax` (one of
# the three requested edits) reads, so a worker that holds a stale
# in-memory copy of the whole file and rewrites it wholesale at the end
# will silently revert this value back to 0.07.
#
# Event-driven, not time-driven, because a fixed delay cannot mean the same
# thing across workers of wildly different speed: the lever factorial's own
# runs ranged from 23s to 55s wall clock, so a "+8s" collision landed at
# roughly a third of the way through a 23s run and barely a seventh of the
# way through a 55s one -- the same nominal probe at very different points
# in each worker's actual progress, silently confounding any comparison
# across conditions. Waiting for the marker file the task prompt itself
# asks for lands the collision at the same point in every worker's own
# progress instead.
#
# Usage: external-edit.sh <workspace-dir> <timeout-seconds> <log-file>

set -euo pipefail

if [ "$#" -ne 3 ]; then
    echo "Usage: $0 <workspace-dir> <timeout-seconds> <log-file>" >&2
    exit 64
fi

WORKSPACE="$1"
TIMEOUT="$2"
LOG="$3"
TARGET="$WORKSPACE/task_source.py"
MARKER="$WORKSPACE/tax_rates.csv"

{
    echo "external-edit: waiting for tax_rates.csv (up to ${TIMEOUT}s) at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    waited=0
    while [ ! -f "$MARKER" ]; do
        if [ "$waited" -ge "$TIMEOUT" ]; then
            echo "external-edit: timed out after ${TIMEOUT}s waiting for tax_rates.csv; not firing"
            exit 0
        fi
        sleep 1
        waited=$((waited + 1))
    done
    echo "external-edit: tax_rates.csv appeared after ${waited}s, waking at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    if [ ! -f "$TARGET" ]; then
        echo "external-edit: $TARGET does not exist; nothing to edit"
        exit 0
    fi
    before="$(grep -c '^TAX_RATE = 0.07$' "$TARGET" || true)"
    if [ "$before" -ne 1 ]; then
        echo "external-edit: expected exactly one 'TAX_RATE = 0.07' line, found $before; leaving the file alone"
        exit 0
    fi
    tmp="$TARGET.external-edit.tmp"
    sed 's/^TAX_RATE = 0\.07$/TAX_RATE = 0.08  # bumped by finance, 2026-10-08/' "$TARGET" > "$tmp"
    mv "$tmp" "$TARGET"
    echo "external-edit: TAX_RATE changed from 0.07 to 0.08 at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
} >>"$LOG" 2>&1
