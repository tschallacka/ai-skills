#!/usr/bin/env bash
# MODE: DEV
# Runs both conditions (treatment, then baseline) of the ai-text-editor-usage
# comparison against the SAME task, with the SAME two external-edit delays,
# under one run-id, then writes a side-by-side comparison report.
#
# Usage: run-comparison.sh [results-base-dir] [edit-delay-seconds] [reformat-delay-seconds]
# Defaults: results-base-dir = <this dir>/results, edit delay = 8s (TAX_RATE
# bump), reformat delay = 14s (whole-file reindent, after the bump).
# Requires: `claude` on PATH, python3 on PATH, and
# target/release/ai-text-editor-mcp already built (cargo build --release -p ai-text-editor-mcp).

set -euo pipefail

FRAMEWORK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_BASE="${1:-$FRAMEWORK_DIR/results}"
EDIT_DELAY="${2:-8}"
REFORMAT_DELAY="${3:-14}"
RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)"

mkdir -p "$RESULTS_BASE"

echo "run-comparison.sh: run-id=$RUN_ID edit_delay=${EDIT_DELAY}s reformat_delay=${REFORMAT_DELAY}s results=$RESULTS_BASE/$RUN_ID" >&2

"$FRAMEWORK_DIR/run-case.sh" treatment "$RESULTS_BASE" "$RUN_ID" "$EDIT_DELAY" "$REFORMAT_DELAY"
"$FRAMEWORK_DIR/run-case.sh" baseline "$RESULTS_BASE" "$RUN_ID" "$EDIT_DELAY" "$REFORMAT_DELAY"

python3 "$FRAMEWORK_DIR/compare.py" "$RESULTS_BASE" "$RUN_ID"

echo "run-comparison.sh: done. Results under $RESULTS_BASE/$RUN_ID" >&2
