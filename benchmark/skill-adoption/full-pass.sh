#!/usr/bin/env bash
# full-pass.sh [CONCURRENCY] -- SUPERSEDED for claude-subject cells.
#
# This script drives `run-cell.sh`, which is itself retired (its
# experimenter step shelled out to `claude -p`, the design a 2026-10-19 full
# pass lost roughly half its cells to -- see PROTOCOL.md's "Why the
# experimenter is a real subagent, not a prompt template"). Running this
# script now just fails fast through `run-cell.sh`'s own refusal. A full
# pass is driven interactively instead: the orchestrating Claude Code
# session partitions the cell matrix into lanes and launches one Agent-tool
# subagent (model haiku) per lane -- see PROTOCOL.md's "Orchestrating a full
# pass". This script is kept only as a reference for the matrix-enumeration
# logic below (the `for scenario_dir ...` loop), not as something to run.
#
# Runs every cell in the default matrix (every scenario's own modes, each
# CLI in $CLIS -- default "claude opencode" -- REPS reps -- default 2) with
# CONCURRENCY cells running at once (default 4), logging each cell's
# stdout/stderr under
# results/logs/<cell-id>.log. One cell failing (a blocked subject, an
# unparseable experimenter bundle -- both expected, logged live-model
# behavior, not harness bugs) does not abort the rest of the matrix: this
# script does not use `set -e`. Aggregates into aggregate.json/AGGREGATE.md
# once every cell has been attempted.
#
# This is dev tooling (benchmark/**), so python3 is used freely alongside
# bash per CODE-STYLE.md's own carve-out, and bash 4.3+ (`wait -n`) is
# assumed rather than the bash-3.2 floor shipped scripts hold to.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/common.sh
. "$HERE/lib/common.sh"

CONCURRENCY="${1:-4}"
REPS="${REPS:-2}"
CLIS="${CLIS:-claude opencode}"

LOG_DIR="$HERE/results/logs"
mkdir -p "$LOG_DIR"

# Sweep any workspace a previous, crashed pass left behind before this one
# launches a single cell -- safe only here, with nothing yet running against
# wa_work_base; see wa_reap_stale_workspaces's own comment.
wa_reap_stale_workspaces

run_one() {
    local id="$1" cli="$2" mode="$3" rep="$4"
    local mode_safe="${mode//\//-}"
    local logfile="$LOG_DIR/${id}_${cli}_${mode_safe}_rep${rep}.log"
    echo "full-pass.sh: START $id $cli $mode rep$rep" >&2
    if "$HERE/run-cell.sh" "$id" "$cli" "$mode" "$rep" > "$logfile" 2>&1; then
        echo "full-pass.sh: OK    $id $cli $mode rep$rep" >&2
    else
        echo "full-pass.sh: FAIL  $id $cli $mode rep$rep (see $logfile)" >&2
    fi
}

total=0
for scenario_dir in "$HERE"/scenarios/S*; do
    id="$(basename "$scenario_dir")"
    modes="$(python3 -c "
import json
print(' '.join(json.load(open('$scenario_dir/scenario.json'))['modes']))
")"
    for cli in $CLIS; do
        for mode in $modes; do
            for rep in $(seq 1 "$REPS"); do
                while [ "$(jobs -rp | wc -l)" -ge "$CONCURRENCY" ]; do
                    wait -n
                done
                run_one "$id" "$cli" "$mode" "$rep" &
                total=$((total + 1))
            done
        done
    done
done
wait

echo "full-pass.sh: all $total cells attempted, aggregating..." >&2
python3 "$HERE/aggregate.py" "$HERE/results/runs.jsonl"
