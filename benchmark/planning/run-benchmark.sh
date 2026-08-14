#!/usr/bin/env bash
# Prepare and run tagged planning-skill benchmarks from a normal user shell.
#
# Usage:
#   benchmark/planning/run-benchmark.sh <name> <testing-base-dir> [--parallel|--sequential] [--iterative|--fresh-review] [--revisions tag[,tag...]] [--versions] [tag ...]

set -euo pipefail

if [ "$#" -lt 2 ]; then
    sed -n '2,6p' "$0" >&2
    exit 64
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Resolve the active benchmark agent (BENCHMARK_AGENT override, default codex)
# and load its driver through the shared runtime launcher.
source "$SCRIPT_DIR/runtime/lib-agent.sh"
RUN_NAME="$1"
TEST_BASE_DIR="$2"
shift
shift

if [[ ! "$RUN_NAME" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; then
    echo "Name must start with a letter or number and contain only letters, numbers, '.', '_' or '-'" >&2
    exit 64
fi

EXECUTION_MODE="${RUN_MODE:-sequential}"
REVIEW_MODE="fresh-review"
REVIEW_MODE_SET=0
REVISIONS=""
MAX_VERIFICATION_PASSES="${MAX_VERIFICATION_PASSES:-3}"
MAX_REVIEW_CYCLES="${MAX_REVIEW_CYCLES:-3}"
INTERACTIVE_VERSIONS=0
EXPLICIT_TAGS=0
TAGS=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        --parallel)
            EXECUTION_MODE="parallel"
            ;;
        --sequential)
            EXECUTION_MODE="sequential"
            ;;
        --versions)
            INTERACTIVE_VERSIONS=1
            ;;
        --iterative|--fresh-review)
            if [ "$REVIEW_MODE_SET" -eq 1 ]; then
                echo "Review mode may be specified only once." >&2
                exit 64
            fi
            REVIEW_MODE="${1#--}"
            REVIEW_MODE_SET=1
            ;;
        --revisions)
            shift
            [ "$#" -gt 0 ] || { echo "--revisions requires tag[,tag...]" >&2; exit 64; }
            REVISIONS="$1"
            [ -n "$REVISIONS" ] || { echo "Revision list may not be empty." >&2; exit 64; }
            ;;
        --revisions=*)
            REVISIONS="${1#--revisions=}"
            [ -n "$REVISIONS" ] || { echo "Revision list may not be empty." >&2; exit 64; }
            ;;
        --*)
            echo "Unknown option: $1" >&2
            exit 64
            ;;
        *)
            TAGS+=("$1")
            EXPLICIT_TAGS=1
            ;;
    esac
    shift
done

if [ -n "$REVISIONS" ]; then
    IFS=',' read -ra revision_tags <<< "$REVISIONS"
    for revision_tag in "${revision_tags[@]}"; do
        [ -n "$revision_tag" ] || { echo "Revision list contains an empty tag." >&2; exit 64; }
        TAGS+=("$revision_tag")
        EXPLICIT_TAGS=1
    done
fi

case "$REVIEW_MODE" in iterative|fresh-review) ;; *) echo "Invalid review mode: $REVIEW_MODE" >&2; exit 64;; esac
[[ "$MAX_VERIFICATION_PASSES" =~ ^[1-9][0-9]*$ ]] || { echo "MAX_VERIFICATION_PASSES must be positive." >&2; exit 64; }
[[ "$MAX_REVIEW_CYCLES" =~ ^[1-9][0-9]*$ ]] || { echo "MAX_REVIEW_CYCLES must be positive." >&2; exit 64; }
export REVIEW_MODE MAX_VERIFICATION_PASSES MAX_REVIEW_CYCLES

RUN_ID="${RUN_ID:-$(date -u +%Y%m%dT%H%M%SZ)-$RUN_NAME}"
if [[ ! "$RUN_ID" =~ ^[0-9]{8}T[0-9]{6}Z-${RUN_NAME//./\.}$ ]]; then
    echo "RUN_ID must have the form UTC_TIMESTAMP-$RUN_NAME" >&2
    exit 64
fi
RESULTS_ROOT="$REPO_ROOT/benchmark/results"
RUN_RESULTS_ROOT="$RESULTS_ROOT/$RUN_ID"
SUMMARY="$RUN_RESULTS_ROOT/harness-summary.tsv"

select_latest_tags() {
    declare -A latest_patch=()
    declare -A latest_tag=()
    declare -A seen_non_semver=()
    local tag key patch
    local -a non_semver_tags=()

    for tag in "$@"; do
        if [[ "$tag" =~ ^v?([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
            key="${BASH_REMATCH[1]}.${BASH_REMATCH[2]}"
            patch="${BASH_REMATCH[3]}"
            if [ -z "${latest_patch[$key]+set}" ] || ((10#$patch > 10#${latest_patch[$key]})); then
                latest_patch["$key"]="$patch"
                latest_tag["$key"]="$tag"
            fi
        else
            if [ -z "${seen_non_semver[$tag]+set}" ]; then
                non_semver_tags+=("$tag")
                seen_non_semver["$tag"]=1
            fi
        fi
    done

    if [ "${#non_semver_tags[@]}" -gt 0 ]; then
        printf '%s\n' "${non_semver_tags[@]}"
    fi
    for key in "${!latest_tag[@]}"; do
        printf '%s\n' "${latest_tag[$key]}"
    done | sort -V
}

choose_versions() {
    local -a available=("$@")
    local -a selected=()
    local -a choices=()
    local selection choice index
    local valid

    if [ ! -t 0 ]; then
        echo "--versions requires an interactive terminal." >&2
        exit 64
    fi

    echo "Available benchmark versions:" >&2
    for index in "${!available[@]}"; do
        printf '  %d) %s\n' "$((index + 1))" "${available[$index]}" >&2
    done
    echo "Select one or more numbers separated by commas (or 'all')." >&2

    while true; do
        if ! IFS= read -r -p 'Versions: ' selection; then
            exit 130
        fi
        if [ "$selection" = "q" ] || [ "$selection" = "quit" ]; then
            exit 130
        fi
        if [ "$selection" = "all" ]; then
            printf '%s\n' "${available[@]}"
            return 0
        fi

        IFS=',' read -ra choices <<< "$selection"
        selected=()
        valid=1
        for choice in "${choices[@]}"; do
            if ! [[ "$choice" =~ ^[0-9]+$ ]] || [ "$choice" -lt 1 ] || [ "$choice" -gt "${#available[@]}" ]; then
                valid=0
                break
            fi
            if [[ ! " ${selected[*]} " =~ " ${available[$((choice - 1))]} " ]]; then
                selected+=("${available[$((choice - 1))]}")
            fi
        done
        if [ "$valid" -eq 1 ] && [ "${#selected[@]}" -gt 0 ]; then
            printf '%s\n' "${selected[@]}"
            return 0
        fi
        echo "Invalid selection; enter numbers from 1 to ${#available[@]}, separated by commas." >&2
    done
}

if [ "$INTERACTIVE_VERSIONS" -eq 1 ]; then
    if [ "$EXPLICIT_TAGS" -eq 1 ]; then
        echo "--versions cannot be combined with explicit tags." >&2
        exit 64
    fi
    mapfile -t TAGS < <(git -C "$REPO_ROOT" tag --sort=version:refname)
    SELECTED_VERSIONS="$(choose_versions "${TAGS[@]}")"
    mapfile -t TAGS <<< "$SELECTED_VERSIONS"
else
    if [ "${#TAGS[@]}" -eq 0 ]; then
        mapfile -t TAGS < <(git -C "$REPO_ROOT" tag --sort=creatordate)
    fi
    mapfile -t TAGS < <(select_latest_tags "${TAGS[@]}")
fi

if [ "${#TAGS[@]}" -eq 0 ]; then
    echo "No tags found to benchmark." >&2
    exit 1
fi

mkdir -p "$TEST_BASE_DIR" "$RUN_RESULTS_ROOT"
printf 'revision\ttag\trun_id\tcase_root\tresult_dir\texit_code\n' > "$SUMMARY"
printf 'run_id\tmode\texecution\tmax_verification_passes\tmax_review_cycles\trevisions\n' > "$RUN_RESULTS_ROOT/harness-metadata.tsv"
printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$RUN_ID" "$REVIEW_MODE" "$EXECUTION_MODE" \
    "$MAX_VERIFICATION_PASSES" "$MAX_REVIEW_CYCLES" "${REVISIONS:-auto}" >> "$RUN_RESULTS_ROOT/harness-metadata.tsv"

START_SCRIPTS=()
for TAG in "${TAGS[@]}"; do
    # Default thresholds to 1.0 so the state synthesizer always receives
    # non-empty values; these defaults match the lifecycle-test values.
    export SEMANTIC_THRESHOLD="${SEMANTIC_THRESHOLD:-1.0}"
    export INDEPENDENT_THRESHOLD="${INDEPENDENT_THRESHOLD:-1.0}"
    "$SCRIPT_DIR/setup-benchmark.sh" "$TAG" "$TEST_BASE_DIR" "$RUN_NAME" "$RUN_ID"
    REVISION="${TAG#v}"
    START_SCRIPTS+=("$TEST_BASE_DIR/$REVISION/start-worker.sh")
done

kill_process_tree() {
    local pid="$1"
    local signal="${2:-TERM}"
    local child

    [ "$pid" -gt 0 ] 2>/dev/null || return 0
    if command -v ps >/dev/null 2>&1; then
        while read -r child; do
            [ -n "$child" ] || continue
            kill_process_tree "$child" "$signal"
        done < <(ps -eo pid=,ppid= | awk -v parent="$pid" '$2 == parent {print $1}')
    fi
    kill -"$signal" "$pid" 2>/dev/null || true
}

WORKER_PIDS=()
ANALYZER_PID=""
cleanup_on_signal() {
    trap - INT TERM

    for pid in "${WORKER_PIDS[@]}"; do
        kill_process_tree "$pid" TERM
    done
    if [ -n "$ANALYZER_PID" ]; then
        kill_process_tree "$ANALYZER_PID" TERM
    fi

    sleep 1

    for pid in "${WORKER_PIDS[@]}"; do
        kill_process_tree "$pid" KILL
    done
    if [ -n "$ANALYZER_PID" ]; then
        kill_process_tree "$ANALYZER_PID" KILL
    fi
    exit 130
}
trap cleanup_on_signal INT TERM

run_case() {
    local start_script="$1"
    local case_root
    local code

    case_root="$(dirname "$start_script")"
    echo "Running $start_script"
    if "$start_script"; then
        code=0
    else
        code="$?"
    fi

    source "$case_root/benchmark-env.sh"
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$REVISION" "$TAG" "$RUN_ID" "$CASE_ROOT" "$RESULT_DIR" "$code" \
        > "$case_root/harness-summary-row.tsv"

    if [ "$code" -ne 0 ]; then
        echo "Worker exited nonzero ($code): $start_script" >&2
        return "$code"
    fi
}

FAILED_WORKERS=0
MAX_PARALLEL_RUNS=5

if [ "$EXECUTION_MODE" = "parallel" ]; then
    for START_SCRIPT in "${START_SCRIPTS[@]}"; do
        run_case "$START_SCRIPT" &
        WORKER_PIDS+=("$!")

        if [ "${#WORKER_PIDS[@]}" -ge "$MAX_PARALLEL_RUNS" ]; then
            if ! wait "${WORKER_PIDS[0]}"; then
                FAILED_WORKERS=1
            fi
            WORKER_PIDS=("${WORKER_PIDS[@]:1}")
        fi
    done

    while [ "${#WORKER_PIDS[@]}" -gt 0 ]; do
        if ! wait "${WORKER_PIDS[0]}"; then
            FAILED_WORKERS=1
        fi
        WORKER_PIDS=("${WORKER_PIDS[@]:1}")
    done
else
    for START_SCRIPT in "${START_SCRIPTS[@]}"; do
        run_case "$START_SCRIPT" &
        WORKER_PIDS=("$!")
        if ! wait "${WORKER_PIDS[0]}"; then
            FAILED_WORKERS=1
        fi
        WORKER_PIDS=()
    done
fi

for START_SCRIPT in "${START_SCRIPTS[@]}"; do
    CASE_ROOT_FOR_SUMMARY="$(dirname "$START_SCRIPT")"
    if [ -f "$CASE_ROOT_FOR_SUMMARY/harness-summary-row.tsv" ]; then
        cat "$CASE_ROOT_FOR_SUMMARY/harness-summary-row.tsv" >> "$SUMMARY"
    else
        REVISION_FOR_SUMMARY="$(basename "$CASE_ROOT_FOR_SUMMARY")"
        printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
            "$REVISION_FOR_SUMMARY" "unavailable" "$RUN_ID" \
            "$CASE_ROOT_FOR_SUMMARY" "$RUN_RESULTS_ROOT/$REVISION_FOR_SUMMARY" \
            "unavailable" >> "$SUMMARY"
        FAILED_WORKERS=1
        echo "Missing harness summary row: $CASE_ROOT_FOR_SUMMARY" >&2
    fi
done

SUMMARY_ROWS="$(tail -n +2 "$SUMMARY" | wc -l | tr -d ' ')"
if [ "$SUMMARY_ROWS" -ne "${#TAGS[@]}" ]; then
    echo "Harness summary has $SUMMARY_ROWS rows for ${#TAGS[@]} benchmark cases" >&2
    FAILED_WORKERS=1
fi

ANALYSIS_ROOT="$RUN_RESULTS_ROOT/analysis"
mkdir -p "$ANALYSIS_ROOT"
ANALYZER_CAPSULE="${PLANNING_AGENT_TMPDIR:-${TMPDIR:-/tmp}/planning-agent}/ai-skills-capsules/$RUN_ID/analysis"
ANALYZER_WORKSPACE="$ANALYSIS_ROOT"
ANALYSIS_RESULTS_ROOT="$ANALYZER_CAPSULE/results"
mkdir -p "$ANALYZER_CAPSULE" "$ANALYSIS_RESULTS_ROOT"
cp "$SCRIPT_DIR/benchmark-test.md" "$ANALYZER_CAPSULE/benchmark-test.md"
cp "$SUMMARY" "$ANALYZER_CAPSULE/harness-summary.tsv"
cp -R "$RUN_RESULTS_ROOT"/. "$ANALYSIS_RESULTS_ROOT"/

sed \
    -e "s/{{RUN_ID}}/$RUN_ID/g" \
    -e "s#{{ANALYSIS_ROOT}}#$ANALYZER_WORKSPACE#g" \
    -e "s#{{RESULTS_ROOT}}#$ANALYSIS_RESULTS_ROOT#g" \
    "$SCRIPT_DIR/analyzer-prompt.md" > "$ANALYZER_CAPSULE/analyzer-prompt.md"
cp "$ANALYZER_CAPSULE/analyzer-prompt.md" "$ANALYSIS_ROOT/analyzer-prompt.md"

echo "Starting analyzer $AGENT_DRIVER session"
set +e
persona_bootstrap analyzer || exit 64
persona_bootstrap_prompt "$ANALYZER_CAPSULE/analyzer-prompt.md" analyzer "$REPO_ROOT/planning/roles/VOICES.md" || exit 64
agent_argv_analyzer "$ANALYZER_WORKSPACE" "$ANALYZER_CAPSULE" "$ANALYZER_CAPSULE/analyzer-prompt.md"
launch_agent background "" "$ANALYSIS_ROOT/analyzer.jsonl"
ANALYZER_PID="$AGENT_PID"
wait_agent
ANALYZER_CODE="$AGENT_EXIT"
ANALYZER_PID=""
set -e

COMPARISON="$RUN_RESULTS_ROOT/comparison.md"
if [ -s "$ANALYSIS_RESULTS_ROOT/comparison.md" ]; then
    cp "$ANALYSIS_RESULTS_ROOT/comparison.md" "$COMPARISON"
fi
if [ ! -s "$COMPARISON" ]; then
    echo "Analyzer did not create a non-empty comparison report: $COMPARISON" >&2
    if [ "$ANALYZER_CODE" -eq 0 ]; then
        ANALYZER_CODE=1
    fi
fi

echo "Benchmark batch complete"
echo "RUN_ID=$RUN_ID"
echo "EXECUTION_MODE=$EXECUTION_MODE"
echo "TEST_BASE_DIR=$TEST_BASE_DIR"
echo "SUMMARY=$SUMMARY"
echo "ANALYZER_EXIT=$ANALYZER_CODE"
echo "WORKERS_FAILED=$FAILED_WORKERS"
echo "COMPARISON=$COMPARISON"

if [ "$ANALYZER_CODE" -ne 0 ]; then
    exit "$ANALYZER_CODE"
fi
if [ "$FAILED_WORKERS" -ne 0 ]; then
    exit 1
fi
