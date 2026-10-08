#!/usr/bin/env bash
# MODE: DEV
# The "someone edits it under your nose" probe: waits a fixed delay (long
# enough for a real worker to have started reading/editing the file, short
# enough to land mid-task rather than after it finishes), then mutates
# task_source.py directly -- outside either condition's own tool, simulating
# a teammate's own concurrent edit. Changes exactly one line: the TAX_RATE
# constant, which `apply_tax` (one of the three requested edits) reads, so a
# worker that holds a stale in-memory copy of the whole file and rewrites it
# wholesale at the end will silently revert this value back to 0.07.
#
# Usage: external-edit.sh <workspace-dir> <delay-seconds> <log-file>

set -euo pipefail

if [ "$#" -ne 3 ]; then
    echo "Usage: $0 <workspace-dir> <delay-seconds> <log-file>" >&2
    exit 64
fi

WORKSPACE="$1"
DELAY="$2"
LOG="$3"
TARGET="$WORKSPACE/task_source.py"

sleep "$DELAY"

{
    echo "external-edit: waking at $(date -u +%Y-%m-%dT%H:%M:%SZ) after ${DELAY}s"
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
