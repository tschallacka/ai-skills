#!/usr/bin/env bash
# MODE: DEV
# Runs one treatment-only case per WORDING VARIANT under wordings/, for each
# of the three author-controlled nudge levers (mcp, edit, read). This is a
# different question than run-hook-matrix-sweep.sh's: that sweep asks
# whether a lever helps at all; this one asks, given a lever is already on,
# which PHRASING of it moves unprompted adoption furthest. Exactly one
# lever is on per run (levers=mcp, edit, or read -- never combined), the
# other two off, so the comparison stays isolated to a single lever's own
# wording. task-prompt.md stays byte-identical across every row, as always
# -- only author-controlled text varies.
#
# Usage: run-wording-sweep.sh [results-base-dir] [edit-timeout-seconds] [reformat-timeout-seconds]
#
# Requires: the same things run-hook-matrix-sweep.sh does, plus the
# wordings/ directory this script reads from -- one *.txt file per variant,
# named "<lever>-<N>-<label>.txt" (lever is "mcp", "edit", or "read").
#
# The edit/read levers depend on editor-gate-plugin's INSTALLED hooks (e.g.
# ~/.claude/skills/editor-gate-plugin/hooks/), not this repo's own copy --
# if that install predates editor_gate_reminder() (FINDINGS.md, "A stale
# installed hook silently disabled the edit/read lever toggle"), every
# edit/read variant below will silently fire the shipped default instead of
# its own wording. Refresh the installed copy from this repo's
# editor-gate-plugin/hooks/ first if in doubt.

set -euo pipefail

FRAMEWORK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_BASE="${1:-$FRAMEWORK_DIR/results}"
EDIT_TIMEOUT="${2:-90}"
REFORMAT_TIMEOUT="${3:-90}"
WORDINGS_DIR="$FRAMEWORK_DIR/wordings"
SWEEP_ID="$(date -u +%Y%m%dT%H%M%SZ)-wordingsweep"
SWEEP_DIR="$RESULTS_BASE/$SWEEP_ID"

mkdir -p "$SWEEP_DIR"

echo "run-wording-sweep.sh: sweep-id=$SWEEP_ID results=$SWEEP_DIR" >&2

for wording_file in "$WORDINGS_DIR"/*.txt; do
    variant_name="$(basename "$wording_file" .txt)"   # e.g. edit-1-terse
    lever="${variant_name%%-*}"                        # mcp / edit / read

    unset MCP_WORDING_OVERRIDE EDIT_WORDING_OVERRIDE READ_WORDING_OVERRIDE
    wording_text="$(cat "$wording_file")"
    case "$lever" in
        mcp)  export MCP_WORDING_OVERRIDE="$wording_text" ;;
        edit) export EDIT_WORDING_OVERRIDE="$wording_text" ;;
        read) export READ_WORDING_OVERRIDE="$wording_text" ;;
        *)
            echo "run-wording-sweep.sh: $wording_file does not start with mcp-/edit-/read-, skipping" >&2
            continue
            ;;
    esac

    echo "run-wording-sweep.sh: --- variant=$variant_name lever=$lever ---" >&2
    "$FRAMEWORK_DIR/run-case.sh" treatment "$SWEEP_DIR" "$variant_name" "$EDIT_TIMEOUT" "$REFORMAT_TIMEOUT" "$lever"
done

python3 "$FRAMEWORK_DIR/compare-wording-sweep.py" "$SWEEP_DIR"

echo "run-wording-sweep.sh: done. Results under $SWEEP_DIR" >&2
