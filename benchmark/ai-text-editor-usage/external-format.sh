#!/usr/bin/env bash
# MODE: DEV
# Second collision surface: a broad, whole-file reindent landing once the
# worker's own departments.csv marker appears -- task-prompt.md's final
# checkpoint, written right before the worker runs the shipped test --
# simulating a teammate's editor doing format-on-save while an agent works
# concurrently (PhpStorm, VS Code -- real, everyday, not contrived). Unlike
# external-edit.sh's single-line TAX_RATE bump, this touches every indented
# line in the file, so it collides with whatever lines the worker is ALSO
# touching, not only a line it never needed to go near.
#
# Event-driven, not time-driven, for the same reason external-edit.sh is:
# a fixed wall-clock delay lands at a different fraction of completion for
# every worker, confounding cross-condition comparison. Waiting for the
# task's own last-checkpoint file lands this collision at the same point
# in every worker's own progress instead.
#
# Usage: external-format.sh <workspace-dir> <timeout-seconds> <log-file>

set -euo pipefail

if [ "$#" -ne 3 ]; then
    echo "Usage: $0 <workspace-dir> <timeout-seconds> <log-file>" >&2
    exit 64
fi

WORKSPACE="$1"
TIMEOUT="$2"
LOG="$3"
TARGET="$WORKSPACE/task_source.py"
MARKER="$WORKSPACE/departments.csv"
FRAMEWORK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

{
    echo "external-format: waiting for departments.csv (up to ${TIMEOUT}s) at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    waited=0
    while [ ! -f "$MARKER" ]; do
        if [ "$waited" -ge "$TIMEOUT" ]; then
            echo "external-format: timed out after ${TIMEOUT}s waiting for departments.csv; not firing"
            exit 0
        fi
        sleep 1
        waited=$((waited + 1))
    done
    echo "external-format: departments.csv appeared after ${waited}s, waking at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    if [ ! -f "$TARGET" ]; then
        echo "external-format: $TARGET does not exist; nothing to reformat"
        exit 0
    fi
    python3 "$FRAMEWORK_DIR/external_reformat.py" "$TARGET"
    echo "external-format: done at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
} >>"$LOG" 2>&1
