#!/usr/bin/env bash
# Prepare and run tagged planning-skill benchmarks from a normal user shell.
#
# Usage:
#   benchmark/planning/run-benchmark.sh <testing-base-dir> [tag ...]

set -euo pipefail

if [ "$#" -lt 1 ]; then
    sed -n '2,6p' "$0" >&2
    exit 64
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TEST_BASE_DIR="$1"
shift

RUN_ID="${RUN_ID:-$(date -u +%Y%m%dT%H%M%SZ)}"
RESULTS_ROOT="$REPO_ROOT/benchmark/results"
SUMMARY="$RESULTS_ROOT/harness-summary-${RUN_ID}.tsv"

if [ "$#" -gt 0 ]; then
    TAGS=("$@")
else
    mapfile -t TAGS < <(git -C "$REPO_ROOT" tag --sort=creatordate)
fi

if [ "${#TAGS[@]}" -eq 0 ]; then
    echo "No tags found to benchmark." >&2
    exit 1
fi

mkdir -p "$TEST_BASE_DIR" "$RESULTS_ROOT"
printf 'revision\ttag\trun_id\tcase_root\tresult_dir\texit_code\n' > "$SUMMARY"

START_SCRIPTS=()
for TAG in "${TAGS[@]}"; do
    "$SCRIPT_DIR/setup-benchmark.sh" "$TAG" "$TEST_BASE_DIR" "$RUN_ID"
    REVISION="${TAG#v}"
    START_SCRIPTS+=("$TEST_BASE_DIR/$REVISION/start-worker.sh")
done

for START_SCRIPT in "${START_SCRIPTS[@]}"; do
    echo "Running $START_SCRIPT"
    set +e
    "$START_SCRIPT"
    CODE="$?"
    set -e

    source "$(dirname "$START_SCRIPT")/benchmark-env.sh"
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$REVISION" "$TAG" "$RUN_ID" "$CASE_ROOT" "$RESULT_DIR" "$CODE" >> "$SUMMARY"

    if [ "$CODE" -ne 0 ]; then
        echo "Worker exited nonzero ($CODE): $START_SCRIPT" >&2
    fi
done

ANALYSIS_ROOT="$TEST_BASE_DIR/analysis-$RUN_ID"
mkdir -p "$ANALYSIS_ROOT"
cp "$SCRIPT_DIR/benchmark-test.md" "$ANALYSIS_ROOT/benchmark-test.md"
cp "$SUMMARY" "$ANALYSIS_ROOT/harness-summary.tsv"

sed \
    -e "s/{{RUN_ID}}/$RUN_ID/g" \
    -e "s#{{ANALYSIS_ROOT}}#$ANALYSIS_ROOT#g" \
    -e "s#{{RESULTS_ROOT}}#$RESULTS_ROOT#g" \
    "$SCRIPT_DIR/analyzer-prompt.md" > "$ANALYSIS_ROOT/analyzer-prompt.md"

echo "Starting analyzer Codex session"
set +e
codex -a never exec --json \
    -C "$ANALYSIS_ROOT" \
    --skip-git-repo-check \
    --sandbox workspace-write \
    --add-dir "$RESULTS_ROOT" \
    "$(cat "$ANALYSIS_ROOT/analyzer-prompt.md")" > "$ANALYSIS_ROOT/analyzer.jsonl" 2>&1
ANALYZER_CODE="$?"
set -e

cp "$ANALYSIS_ROOT/analyzer.jsonl" "$RESULTS_ROOT/analyzer-${RUN_ID}.jsonl"

echo "Benchmark batch complete"
echo "RUN_ID=$RUN_ID"
echo "TEST_BASE_DIR=$TEST_BASE_DIR"
echo "SUMMARY=$SUMMARY"
echo "ANALYZER_EXIT=$ANALYZER_CODE"
echo "COMPARISON=$RESULTS_ROOT/comparison-${RUN_ID}.md"
