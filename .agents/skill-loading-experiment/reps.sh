#!/usr/bin/env bash
# MODE: DEV
# Runs N replicates of one probe condition and appends each TSV row to a log.
#
# Repetition is the point: the truncation itself proved deterministic, but
# whether the agent notices and chunk-reads the remainder did not, so a single
# run of any condition is an anecdote.
#
# Usage: reps.sh <count> <label-prefix>   (other settings come from the env
# that probe-claude-cap.sh already reads: SRC, MODEL, STYLE, OUTDIR)
set -euo pipefail

count="${1:?usage: reps.sh <count> <label-prefix>}"
prefix="${2:?usage: reps.sh <count> <label-prefix>}"
here="$(cd "$(dirname "$0")" && pwd)"
log="${LOG:-${OUTDIR:?OUTDIR must be set}/results.tsv}"

index=1
while [ "$index" -le "$count" ]; do
    LABEL="${prefix}-r${index}" bash "$here/probe-claude-cap.sh" | tee -a "$log"
    index=$((index + 1))
done
