#!/usr/bin/env bash
# MODE: DEV
# Runs the TREATMENT condition only (ai-text-editor-mcp available), once per
# combination of the three levers run-case.sh exposes -- the MCP server's own
# top-level instructions pitch, and editor-gate-plugin's Edit/Write and Read
# PreToolUse hooks -- the full 2^3 factorial, 8 runs. task-prompt.md is
# byte-identical across every row: the only things that vary are things the
# TOOL AUTHOR controls, never anything an end user would have to type into
# the prompt. This is the question the whole benchmark exists to answer --
# which combination of author-controlled levers actually gets an agent to
# reach for ai-text-editor unprompted, and at what cost (correctness,
# tokens, wall clock).
#
# Usage: run-hook-matrix-sweep.sh [results-base-dir] [edit-timeout-seconds] [reformat-timeout-seconds]
# Both collisions fire as soon as the worker's own tax_rates.csv /
# departments.csv checkpoint appears, not on a fixed delay -- the timeouts
# here only bound how long to wait before giving up on one; see
# run-case.sh's own header for why.
# Requires: `claude` on PATH, python3 on PATH, editor-gate-plugin installed
# (its hooks must actually fire for this sweep to measure anything), and
# target/release/ai-text-editor-mcp already built with the
# AI_TEXT_EDITOR_MCP_INSTRUCTIONS env-var hook (cargo build --release -p
# ai-text-editor-mcp).

set -euo pipefail

FRAMEWORK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_BASE="${1:-$FRAMEWORK_DIR/results}"
EDIT_TIMEOUT="${2:-90}"
REFORMAT_TIMEOUT="${3:-90}"
SWEEP_ID="$(date -u +%Y%m%dT%H%M%SZ)-leverfactorial"
SWEEP_DIR="$RESULTS_BASE/$SWEEP_ID"

mkdir -p "$SWEEP_DIR"

echo "run-hook-matrix-sweep.sh: sweep-id=$SWEEP_ID results=$SWEEP_DIR" >&2

# Every combination of {mcp, edit, read}, each independently on or off --
# case-dir names use "-" (filesystem-friendly); the real "+"-joined form
# goes to run-case.sh's own [levers] argument.
for levers in none mcp edit read mcp+edit mcp+read edit+read mcp+edit+read; do
    case_name="${levers//+/-}"
    echo "run-hook-matrix-sweep.sh: --- levers=$levers ---" >&2
    "$FRAMEWORK_DIR/run-case.sh" treatment "$SWEEP_DIR" "$case_name" "$EDIT_TIMEOUT" "$REFORMAT_TIMEOUT" "$levers"
done

python3 "$FRAMEWORK_DIR/compare-hook-matrix.py" "$SWEEP_DIR"

echo "run-hook-matrix-sweep.sh: done. Results under $SWEEP_DIR" >&2
