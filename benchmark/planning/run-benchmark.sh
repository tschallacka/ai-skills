#!/usr/bin/env bash
# Prepare and run tagged planning-skill benchmarks from a normal user shell.
#
# Usage:
#   benchmark/planning/run-benchmark.sh <name> <testing-base-dir> [--parallel|--sequential] [--iterative|--fresh-review] [--revisions tag[,tag...]] [--versions] [tag ...]
#
# Agent time budgets, all overridable: WORKER_TIMEOUT (45m), REVIEWER_TIMEOUT
# (20m), ANALYZER_TIMEOUT (30m). The analyzer's exit code gates the batch, so its
# bound is what stops a hung analyzer from hanging an already-paid-for run.

set -euo pipefail

if [ "$#" -lt 2 ]; then
    awk 'NR == 1 { next }
         /^#/ {
             sub(/^#[[:space:]]?/, "")
             if ($0 ~ /^----[[:space:]]*(quoted:|end quoted)/) next
             print; next
         }
         { exit }' "$0" >&2
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

EXECUTION_MODE=""
RUN_MODE_SET=0
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
            RUN_MODE_SET=1
            ;;
        --sequential)
            EXECUTION_MODE="sequential"
            RUN_MODE_SET=1
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

if [ "$RUN_MODE_SET" -eq 0 ]; then
    if [ -n "${RUN_MODE:-}" ]; then
        EXECUTION_MODE="$RUN_MODE"
    elif [ -t 0 ]; then
        printf 'Run benchmarks sequentially or in parallel? [sequential]: ' >&2
        read -r mode_answer
        # bash 3.2 has no ${var,,}; the rest of this runtime already uses tr.
        case "$(printf '%s' "$mode_answer" | tr '[:upper:]' '[:lower:]')" in
            p*) EXECUTION_MODE="parallel" ;;
            ""|s*) EXECUTION_MODE="sequential" ;;
            *)
                echo "Unknown mode '$mode_answer'; using sequential" >&2
                EXECUTION_MODE="sequential"
                ;;
        esac
    else
        EXECUTION_MODE="sequential"
    fi
fi
case "$EXECUTION_MODE" in
    sequential|parallel) ;;
    *) echo "Invalid execution mode: $EXECUTION_MODE" >&2; exit 64 ;;
esac

case "$REVIEW_MODE" in iterative|fresh-review) ;; *) echo "Invalid review mode: $REVIEW_MODE" >&2; exit 64;; esac
[[ "$MAX_VERIFICATION_PASSES" =~ ^[1-9][0-9]*$ ]] || { echo "MAX_VERIFICATION_PASSES must be positive." >&2; exit 64; }
[[ "$MAX_REVIEW_CYCLES" =~ ^[1-9][0-9]*$ ]] || { echo "MAX_REVIEW_CYCLES must be positive." >&2; exit 64; }
export REVIEW_MODE MAX_VERIFICATION_PASSES MAX_REVIEW_CYCLES

RUN_ID="${RUN_ID:-$(date -u +%Y%m%dT%H%M%SZ)-$RUN_NAME}"
if [[ ! "$RUN_ID" =~ ^[0-9]{8}T[0-9]{6}Z-${RUN_NAME//./\.}$ ]]; then
    echo "RUN_ID must have the form UTC_TIMESTAMP-$RUN_NAME" >&2
    exit 64
fi
# Tailable progress log for the invoking process: tail this file to watch
# stage transitions (preflight, worker, validation, review, oracle, publish).
PROGRESS_LOG="${PROGRESS_LOG:-${TMPDIR:-/tmp}/ai-skills-benchmark-progress-$RUN_ID.log}"
export PROGRESS_LOG
progress_log() {
    printf '[%s] %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" | tee -a "$PROGRESS_LOG"
}
progress_log "benchmark run $RUN_ID started (mode=$EXECUTION_MODE review=$REVIEW_MODE revisions=${REVISIONS:-auto})"
# RUN_RESULTS_ROOT must wait for tag resolution below: computed here it reads an
# empty TAGS and splits the results tree in two.
RESULTS_ROOT="$REPO_ROOT/benchmark/results/$AGENT_DRIVER"

# select_latest_tags <tag...>: print every non-semver tag, then the newest patch
# of each major.minor, ascending. bash 3.2 has no associative arrays, so keys and
# winners live in parallel arrays with a linear lookup.
select_latest_tags() {
    local -a keys=() winners=() patches=() non_semver=()
    local tag key patch index found

    for tag in "$@"; do
        if [[ "$tag" =~ ^v?([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
            key="${BASH_REMATCH[1]}.${BASH_REMATCH[2]}"
            patch="${BASH_REMATCH[3]}"
            found=-1
            index=0
            while [ "$index" -lt "${#keys[@]}" ]; do
                if [ "${keys[$index]}" = "$key" ]; then
                    found="$index"
                    break
                fi
                index=$((index + 1))
            done
            if [ "$found" -lt 0 ]; then
                keys+=("$key")
                winners+=("$tag")
                patches+=("$patch")
            elif [ "$((10#$patch))" -gt "$((10#${patches[$found]}))" ]; then
                winners[found]="$tag"
                patches[found]="$patch"
            fi
        else
            case " ${non_semver[*]-} " in
                *" $tag "*) ;;
                *) non_semver+=("$tag") ;;
            esac
        fi
    done

    if [ "${#non_semver[@]}" -gt 0 ]; then
        printf '%s\n' "${non_semver[@]}"
    fi
    if [ "${#winners[@]}" -gt 0 ]; then
        # `sort -V` is GNU-only; sort on a zero-padded numeric key instead.
        printf '%s\n' "${winners[@]}" |
            awk '{ tag = $0; version = tag; sub(/^v/, "", version)
                   split(version, part, ".")
                   printf "%010d.%010d.%010d\t%s\n", part[1], part[2], part[3], tag }' |
            LC_ALL=C sort |
            cut -f2-
    fi
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
            already_selected=0
            for selected_tag in "${selected[@]}"; do
                if [ "$selected_tag" = "${available[$((choice - 1))]}" ]; then
                    already_selected=1
                    break
                fi
            done
            if [ "$already_selected" -eq 0 ]; then
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
    TAGS=()
    while IFS= read -r tag_line; do
        [ -n "$tag_line" ] || continue
        TAGS+=("$tag_line")
    done < <(git -C "$REPO_ROOT" tag --sort=version:refname)
    SELECTED_VERSIONS="$(choose_versions ${TAGS[@]+"${TAGS[@]}"})"
    TAGS=()
    while IFS= read -r tag_line; do
        [ -n "$tag_line" ] || continue
        TAGS+=("$tag_line")
    done <<TAG_LIST
$SELECTED_VERSIONS
TAG_LIST
else
    if [ "${#TAGS[@]}" -eq 0 ]; then
        while IFS= read -r tag_line; do
            [ -n "$tag_line" ] || continue
            TAGS+=("$tag_line")
        done < <(git -C "$REPO_ROOT" tag --sort=creatordate)
    fi
    SELECTED_VERSIONS="$(select_latest_tags ${TAGS[@]+"${TAGS[@]}"})"
    TAGS=()
    while IFS= read -r tag_line; do
        [ -n "$tag_line" ] || continue
        TAGS+=("$tag_line")
    done <<TAG_LIST
$SELECTED_VERSIONS
TAG_LIST
fi

if [ "${#TAGS[@]}" -eq 0 ]; then
    echo "No tags found to benchmark." >&2
    exit 1
fi

# Run-level files stay at the first tag's run-id root, so a batch spanning
# several revisions is still one batch; each case's own published directory goes
# in CASE_RESULT_DIRS, since it lives under its own revision parent.
RUN_RESULTS_ROOT="$RESULTS_ROOT/$(benchmark_result_parent "${TAGS[0]}")/$RUN_ID"
SUMMARY="$RUN_RESULTS_ROOT/harness-summary.tsv"

mkdir -p "$TEST_BASE_DIR" "$RUN_RESULTS_ROOT"
printf 'revision\ttag\trun_id\tcase_root\tresult_dir\texit_code\n' > "$SUMMARY"
printf 'run_id\tmode\texecution\tmax_verification_passes\tmax_review_cycles\trevisions\n' > "$RUN_RESULTS_ROOT/harness-metadata.tsv"
printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$RUN_ID" "$REVIEW_MODE" "$EXECUTION_MODE" \
    "$MAX_VERIFICATION_PASSES" "$MAX_REVIEW_CYCLES" "${REVISIONS:-auto}" >> "$RUN_RESULTS_ROOT/harness-metadata.tsv"

START_SCRIPTS=()
CASE_RESULT_DIRS=()
for TAG in "${TAGS[@]}"; do
    # Default thresholds to 1.0 so the state synthesizer always receives
    # non-empty values; these defaults match the lifecycle-test values.
    export SEMANTIC_THRESHOLD="${SEMANTIC_THRESHOLD:-1.0}"
    export INDEPENDENT_THRESHOLD="${INDEPENDENT_THRESHOLD:-1.0}"
    "$SCRIPT_DIR/setup-benchmark.sh" "$TAG" "$TEST_BASE_DIR" "$RUN_NAME" "$RUN_ID"
    REVISION="${TAG#v}"
    START_SCRIPTS+=("$TEST_BASE_DIR/$REVISION-$RUN_ID/start-worker.sh")
    # Mirrors setup-benchmark.sh's RESULT_DIR exactly, so the analyzer input and
    # the fallback summary row point at the tree the case really publishes to.
    CASE_RESULT_DIRS+=("$RESULTS_ROOT/$(benchmark_result_parent "$TAG")/$RUN_ID/$REVISION")
    progress_log "case prepared: $TAG -> $TEST_BASE_DIR/$REVISION-$RUN_ID"
done

# kill_process_tree comes from runtime/lib-agent.sh (sourced above); this file
# used to carry a third verbatim copy of it.
WORKER_PIDS=()
ANALYZER_PID=""
cleanup_on_signal() {
    trap - INT TERM

    for pid in ${WORKER_PIDS[@]+"${WORKER_PIDS[@]}"}; do
        kill_process_tree "$pid" TERM
    done
    if [ -n "$ANALYZER_PID" ]; then
        kill_process_tree "$ANALYZER_PID" TERM
    fi

    sleep 1

    for pid in ${WORKER_PIDS[@]+"${WORKER_PIDS[@]}"}; do
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
    progress_log "starting worker: $start_script"
    echo "Running $start_script"
    if "$start_script"; then
        code=0
    else
        code="$?"
    fi
    progress_log "worker finished: $start_script (code=$code)"

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

CASE_INDEX=0
while [ "$CASE_INDEX" -lt "${#START_SCRIPTS[@]}" ]; do
    CASE_ROOT_FOR_SUMMARY="$(dirname "${START_SCRIPTS[$CASE_INDEX]}")"
    if [ -f "$CASE_ROOT_FOR_SUMMARY/harness-summary-row.tsv" ]; then
        cat "$CASE_ROOT_FOR_SUMMARY/harness-summary-row.tsv" >> "$SUMMARY"
    else
        REVISION_FOR_SUMMARY="$(basename "$CASE_ROOT_FOR_SUMMARY")"
        REVISION_FOR_SUMMARY="${REVISION_FOR_SUMMARY%-$RUN_ID}"
        printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
            "$REVISION_FOR_SUMMARY" "unavailable" "$RUN_ID" \
            "$CASE_ROOT_FOR_SUMMARY" "${CASE_RESULT_DIRS[$CASE_INDEX]}" \
            "unavailable" >> "$SUMMARY"
        FAILED_WORKERS=1
        echo "Missing harness summary row: $CASE_ROOT_FOR_SUMMARY" >&2
    fi
    CASE_INDEX=$((CASE_INDEX + 1))
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
# `cp -R src/. dst` has unspecified behaviour with a trailing `/.` (CODE-STYLE
# §1); tar copies the tree portably.
copy_tree() {
    local src="$1" dst="$2"
    [ -d "$src" ] || return 0
    mkdir -p "$dst"
    (cd "$src" && tar cf - .) | (cd "$dst" && tar xf -)
}
copy_tree "$RUN_RESULTS_ROOT" "$ANALYSIS_RESULTS_ROOT"
# Only the first tag's case publishes under RUN_RESULTS_ROOT; every other
# revision lives under its own revision parent and would otherwise reach the
# analyzer as no case results at all.
for CASE_RESULT_DIR in "${CASE_RESULT_DIRS[@]}"; do
    case "$CASE_RESULT_DIR" in
        "$RUN_RESULTS_ROOT"/*) continue ;;
    esac
    copy_tree "$CASE_RESULT_DIR" "$ANALYSIS_RESULTS_ROOT/$(basename "$CASE_RESULT_DIR")"
done

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
# Bounded like the worker and the reviewer: ANALYZER_CODE gates the batch's exit
# code, so an unbounded analyzer hangs the batch after every agent has been paid.
launch_agent background "${ANALYZER_TIMEOUT:-30m}" "$ANALYSIS_ROOT/analyzer.jsonl"
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
