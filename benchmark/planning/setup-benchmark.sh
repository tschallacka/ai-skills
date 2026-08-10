#!/usr/bin/env bash
# Prepare one tagged planning benchmark case.
#
# Usage:
#   benchmark/planning/setup-benchmark.sh <tag> <testing-base-dir> [run-id]

set -euo pipefail

if [ "$#" -lt 2 ] || [ "$#" -gt 3 ]; then
    sed -n '2,6p' "$0" >&2
    exit 64
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

TAG="$1"
TEST_BASE_DIR="$2"
RUN_ID="${3:-$(date -u +%Y%m%dT%H%M%SZ)}"
REVISION="${TAG#v}"
CASE_ROOT="$TEST_BASE_DIR/$REVISION"
SRC_ROOT="$CASE_ROOT/source"
BENCH_ROOT="$CASE_ROOT/workspace"
PLAN_NAME="basic-test-proof-${REVISION}-${RUN_ID}-isolated-plan"
RESULT_DIR="$REPO_ROOT/benchmark/results/$REVISION/$RUN_ID"

escape_sed_replacement() {
    printf '%s' "$1" | sed -e 's/[\/&]/\\&/g'
}

render_template() {
    local template="$1"
    local output="$2"
    sed \
        -e "s/{{REVISION}}/$(escape_sed_replacement "$REVISION")/g" \
        -e "s/{{RUN_ID}}/$(escape_sed_replacement "$RUN_ID")/g" \
        -e "s/{{SRC_ROOT}}/$(escape_sed_replacement "$SRC_ROOT")/g" \
        -e "s/{{BENCH_ROOT}}/$(escape_sed_replacement "$BENCH_ROOT")/g" \
        -e "s/{{PLAN_NAME}}/$(escape_sed_replacement "$PLAN_NAME")/g" \
        "$template" > "$output"
}

if ! git -C "$REPO_ROOT" rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
    echo "Tag not found: $TAG" >&2
    exit 66
fi

if [ -e "$CASE_ROOT" ]; then
    echo "Benchmark case already exists: $CASE_ROOT" >&2
    exit 73
fi

mkdir -p "$SRC_ROOT" "$BENCH_ROOT" "$RESULT_DIR"
git -C "$REPO_ROOT" archive "$TAG" | tar -x -C "$SRC_ROOT"

if [ ! -f "$SRC_ROOT/basic-test-proof-plan.md" ]; then
    cp "$SCRIPT_DIR/task-spec.md" "$SRC_ROOT/basic-test-proof-plan.md"
fi

cp "$SCRIPT_DIR/benchmark-test.md" "$BENCH_ROOT/benchmark-test.md"
cp "$SCRIPT_DIR/task-spec.md" "$BENCH_ROOT/task-spec.md"
cp "$SCRIPT_DIR/telemetry.sh" "$CASE_ROOT/telemetry.sh"
cp "$SCRIPT_DIR/session-id-from-jsonl.sh" "$CASE_ROOT/session-id-from-jsonl.sh"
render_template "$SCRIPT_DIR/worker-prompt.md" "$BENCH_ROOT/worker-prompt.md"

cat > "$CASE_ROOT/benchmark-env.sh" <<EOF
#!/usr/bin/env bash
export REPO_ROOT=$(printf '%q' "$REPO_ROOT")
export TAG=$(printf '%q' "$TAG")
export REVISION=$(printf '%q' "$REVISION")
export RUN_ID=$(printf '%q' "$RUN_ID")
export CASE_ROOT=$(printf '%q' "$CASE_ROOT")
export SRC_ROOT=$(printf '%q' "$SRC_ROOT")
export BENCH_ROOT=$(printf '%q' "$BENCH_ROOT")
export PLAN_NAME=$(printf '%q' "$PLAN_NAME")
export RESULT_DIR=$(printf '%q' "$RESULT_DIR")
EOF

cat > "$CASE_ROOT/start-worker.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/benchmark-env.sh"

START_EPOCH="$(date -u +%s)"
START="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

find "$BENCH_ROOT" -maxdepth 1 -mindepth 1 -printf '%f\n' | sort > "$CASE_ROOT/preflight-before-worker.txt"

set +e
timeout "${WORKER_TIMEOUT:-45m}" codex -a never exec --json \
    -C "$BENCH_ROOT" \
    --skip-git-repo-check \
    --sandbox workspace-write \
    --add-dir "$SRC_ROOT" \
    "$(cat "$BENCH_ROOT/worker-prompt.md")" > "$BENCH_ROOT/worker.jsonl" 2>&1
CODE="$?"
set -e

END_EPOCH="$(date -u +%s)"
END="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
ELAPSED="$((END_EPOCH - START_EPOCH))"

if [ ! -s "$BENCH_ROOT/session-id.txt" ]; then
    "$(dirname "$0")/session-id-from-jsonl.sh" "$BENCH_ROOT/worker.jsonl" > "$BENCH_ROOT/session-id.txt"
    echo "session_id_source=worker.jsonl" > "$BENCH_ROOT/session-id-source.txt"
else
    echo "session_id_source=worker" > "$BENCH_ROOT/session-id-source.txt"
fi

SESSION_ID="$(tr -d '[:space:]' < "$BENCH_ROOT/session-id.txt")"
[ -n "$SESSION_ID" ] || SESSION_ID="unavailable"

"$(dirname "$0")/telemetry.sh" "$SESSION_ID" > "$BENCH_ROOT/telemetry.txt"

VALIDATION="not-run-or-not-found"
if [ -x "$SRC_ROOT/planning/scripts/validate-plan.sh" ] && [ -d "$BENCH_ROOT/$PLAN_NAME" ]; then
    if "$SRC_ROOT/planning/scripts/validate-plan.sh" "$BENCH_ROOT/$PLAN_NAME" > "$BENCH_ROOT/harness-validation.txt" 2>&1; then
        VALIDATION="pass"
    else
        VALIDATION="fail"
    fi
fi

USAGE_RECORDS="$(sed -n 's/^usage_records=//p' "$BENCH_ROOT/telemetry.txt" | head -1)"
TOTAL_USAGE="$(sed -n 's/^total_usage_tokens=//p' "$BENCH_ROOT/telemetry.txt" | head -1)"
HTML_COUNT="$(find "$BENCH_ROOT" -type f \( -name '*.html' -o -name '*.htm' \) | wc -l | tr -d ' ')"
GOALS="$(find "$BENCH_ROOT/$PLAN_NAME" -type f -name 'goal.md' 2>/dev/null | wc -l | tr -d ' ')"
WORK_UNITS="$(find "$BENCH_ROOT/$PLAN_NAME" -type f \( -name '*work-unit*.md' -o -name 'work-unit-inventory.md' -o -name 'atomic-work-unit*.md' \) 2>/dev/null | wc -l | tr -d ' ')"

STATUS="accepted"
if [ "$CODE" -ne 0 ] || [ ! -d "$BENCH_ROOT/$PLAN_NAME" ] || [ "$HTML_COUNT" != 0 ] || [ "$VALIDATION" = "fail" ]; then
    STATUS="tainted"
fi
if [ "$SESSION_ID" = "unavailable" ] || [ "${USAGE_RECORDS:-0}" = "0" ] || [ "${USAGE_RECORDS:-unavailable}" = "unavailable" ]; then
    STATUS="tainted"
fi

mkdir -p "$RESULT_DIR"
cp -R "$BENCH_ROOT"/. "$RESULT_DIR"/
cp "$CASE_ROOT/preflight-before-worker.txt" "$RESULT_DIR/preflight-before-worker.txt"

cat > "$RESULT_DIR/evaluation.md" <<EVAL
# Benchmark evaluation $REVISION $RUN_ID

- Status: $STATUS
- Revision: $REVISION ($TAG)
- Isolated directory: $BENCH_ROOT
- Tagged source directory: $SRC_ROOT
- Result archive: $RESULT_DIR
- Plan: $PLAN_NAME
- Worker exit code: $CODE
- Session ID: $SESSION_ID
- Telemetry records: ${USAGE_RECORDS:-unavailable}
- Total usage tokens: ${TOTAL_USAGE:-unavailable}
- Start: $START
- End: $END
- Elapsed seconds: $ELAPSED
- Work-unit count: $WORK_UNITS
- Goal count: $GOALS
- Validation result: $VALIDATION
- Artifact/process audit result: html_or_htm_files=$HTML_COUNT
EVAL

printf 'completed %s code=%s status=%s result=%s\n' "$REVISION" "$CODE" "$STATUS" "$RESULT_DIR"
exit "$CODE"
EOF

chmod +x \
    "$CASE_ROOT/benchmark-env.sh" \
    "$CASE_ROOT/start-worker.sh" \
    "$CASE_ROOT/telemetry.sh" \
    "$CASE_ROOT/session-id-from-jsonl.sh"

cat <<EOF
Prepared benchmark case
  tag:        $TAG
  revision:   $REVISION
  run id:     $RUN_ID
  case root:  $CASE_ROOT
  source:     $SRC_ROOT
  workspace:  $BENCH_ROOT
  start:      $CASE_ROOT/start-worker.sh
  result:     $RESULT_DIR
EOF
