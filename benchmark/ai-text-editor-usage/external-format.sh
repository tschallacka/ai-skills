#!/usr/bin/env bash
# MODE: DEV
# Second collision surface: a broad, whole-file reindent landing mid-task,
# simulating a teammate's editor doing format-on-save while an agent works
# concurrently (PhpStorm, VS Code -- real, everyday, not contrived). Unlike
# external-edit.sh's single-line TAX_RATE bump, this touches every indented
# line in the file, so it collides with whatever lines the worker is ALSO
# touching, not only a line it never needed to go near.
#
# Usage: external-format.sh <workspace-dir> <delay-seconds> <log-file>

set -euo pipefail

if [ "$#" -ne 3 ]; then
    echo "Usage: $0 <workspace-dir> <delay-seconds> <log-file>" >&2
    exit 64
fi

WORKSPACE="$1"
DELAY="$2"
LOG="$3"
TARGET="$WORKSPACE/task_source.py"
FRAMEWORK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

sleep "$DELAY"

{
    echo "external-format: waking at $(date -u +%Y-%m-%dT%H:%M:%SZ) after ${DELAY}s"
    if [ ! -f "$TARGET" ]; then
        echo "external-format: $TARGET does not exist; nothing to reformat"
        exit 0
    fi
    python3 "$FRAMEWORK_DIR/external_reformat.py" "$TARGET"
    echo "external-format: done at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
} >>"$LOG" 2>&1
