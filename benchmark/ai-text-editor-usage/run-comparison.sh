#!/usr/bin/env bash
# MODE: DEV
# Runs both conditions (treatment, then baseline) of the ai-text-editor-usage
# comparison against the SAME task, with the SAME two external-edit
# checkpoint-wait timeouts, under one run-id, then writes a side-by-side
# comparison report.
#
# Usage: run-comparison.sh [results-base-dir] [edit-timeout-seconds] [reformat-timeout-seconds]
# Defaults: results-base-dir = <this dir>/results, edit timeout = 90s (how
# long to wait for tax_rates.csv before giving up on the TAX_RATE bump),
# reformat timeout = 90s (how long to wait for departments.csv before giving
# up on the whole-file reindent). Both collisions fire as soon as their
# marker file appears, not on a fixed delay -- see run-case.sh's own header.
# Requires: `claude` on PATH, python3 on PATH, and
# target/release/ai-text-editor-mcp already built (cargo build --release -p ai-text-editor-mcp).

set -euo pipefail

FRAMEWORK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_BASE="${1:-$FRAMEWORK_DIR/results}"
EDIT_TIMEOUT="${2:-90}"
REFORMAT_TIMEOUT="${3:-90}"
RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)"

mkdir -p "$RESULTS_BASE"

echo "run-comparison.sh: run-id=$RUN_ID edit_timeout=${EDIT_TIMEOUT}s reformat_timeout=${REFORMAT_TIMEOUT}s results=$RESULTS_BASE/$RUN_ID" >&2

"$FRAMEWORK_DIR/run-case.sh" treatment "$RESULTS_BASE" "$RUN_ID" "$EDIT_TIMEOUT" "$REFORMAT_TIMEOUT"
"$FRAMEWORK_DIR/run-case.sh" baseline "$RESULTS_BASE" "$RUN_ID" "$EDIT_TIMEOUT" "$REFORMAT_TIMEOUT"

python3 "$FRAMEWORK_DIR/compare.py" "$RESULTS_BASE" "$RUN_ID"

echo "run-comparison.sh: done. Results under $RESULTS_BASE/$RUN_ID" >&2
