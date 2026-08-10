#!/usr/bin/env bash
# Prepare one tagged planning benchmark case.
#
# Usage:
#   benchmark/planning/setup-benchmark.sh <tag> <testing-base-dir> <name> [run-id]

set -euo pipefail

if [ "$#" -lt 3 ] || [ "$#" -gt 4 ]; then
    sed -n '2,6p' "$0" >&2
    exit 64
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

TAG="$1"
TEST_BASE_DIR="$2"
RUN_NAME="$3"
RUN_ID="${4:-$(date -u +%Y%m%dT%H%M%SZ)-$RUN_NAME}"
REVISION="${TAG#v}"
CASE_ROOT="$TEST_BASE_DIR/$REVISION"
SRC_ROOT="$CASE_ROOT/source"
BENCH_ROOT="$CASE_ROOT/workspace"
PLAN_NAME="basic-test-proof-${REVISION}-${RUN_ID}-isolated-plan"
RESULT_DIR="$REPO_ROOT/benchmark/results/$RUN_ID/$REVISION"

if [[ ! "$RUN_NAME" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; then
    echo "Name must start with a letter or number and contain only letters, numbers, '.', '_' or '-'" >&2
    exit 64
fi
if [[ ! "$RUN_ID" =~ ^[0-9]{8}T[0-9]{6}Z-${RUN_NAME//./\.}$ ]]; then
    echo "Run ID must have the form UTC_TIMESTAMP-$RUN_NAME" >&2
    exit 64
fi

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

COMPATIBILITY_REPORT="$BENCH_ROOT/benchmark-compatibility.txt"
printf 'Benchmark compatibility adjustments for %s\n\n' "$TAG" > "$COMPATIBILITY_REPORT"
for CONTEXT_TEST in \
    "$SRC_ROOT/planning/tests/test-plan-context.sh" \
    "$SRC_ROOT/planning/tests/test-plan-context-deferred-boundary.sh"; do
    if [ ! -f "$CONTEXT_TEST" ] || ! grep -Fq '/home/tschallacka/.codex/skills/planning/plans/planning-context-cache' "$CONTEXT_TEST"; then
        continue
    fi

    sed -i '/^cp -R \/home\/tschallacka\/.codex\/skills\/planning\/plans\/planning-context-cache/c\
context_cache="${PLANNING_CONTEXT_CACHE:-${HOME}/.codex/skills/planning/plans/planning-context-cache}"\
if [ ! -d "$context_cache" ]; then\
    printf '\''test-plan-context: SKIPPED (fixture unavailable: %s)\\n'\'' "$context_cache"\
    exit 0\
fi\
cp -R "$context_cache" "$tmp/plan"' "$CONTEXT_TEST"
    printf '%s\n' "Patched ${CONTEXT_TEST#"$SRC_ROOT"/}: user-specific fixture path is now overrideable and skips cleanly when unavailable." >> "$COMPATIBILITY_REPORT"
done

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

PROCESS_PATTERN='(google-chrome|chromium|firefox|playwright|geckodriver|chromedriver|selenium|http\.server|php -S|vite|webpack-dev-server|node.*(serve|vite)|npm.*(run|exec).*(dev|serve))'
PROCESS_AUDIT_STATE="$CASE_ROOT/process-audit-state.txt"
if command -v ps >/dev/null 2>&1 && command -v setsid >/dev/null 2>&1; then
    echo "available" > "$PROCESS_AUDIT_STATE"
else
    echo "unavailable:ps-and-setsid-required" > "$PROCESS_AUDIT_STATE"
fi

find "$BENCH_ROOT" -maxdepth 1 -mindepth 1 -printf '%f\n' | sort > "$CASE_ROOT/preflight-before-worker.txt"

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

WORKER_CHILD_PID=""
WORKER_PROCESS_GROUP_ID=""
cleanup_on_signal() {
    trap - INT TERM
    if [ -n "$WORKER_CHILD_PID" ]; then
        if [ -n "$WORKER_PROCESS_GROUP_ID" ]; then
            kill -TERM -- "-$WORKER_PROCESS_GROUP_ID" 2>/dev/null || true
        fi
        kill_process_tree "$WORKER_CHILD_PID" TERM
        sleep 1
        if [ -n "$WORKER_PROCESS_GROUP_ID" ]; then
            kill -KILL -- "-$WORKER_PROCESS_GROUP_ID" 2>/dev/null || true
        fi
        kill_process_tree "$WORKER_CHILD_PID" KILL
    fi
    exit 130
}
trap cleanup_on_signal INT TERM

setsid timeout "${WORKER_TIMEOUT:-45m}" codex -a never exec --json \
    -C "$BENCH_ROOT" \
    --skip-git-repo-check \
    --sandbox workspace-write \
    --add-dir "$SRC_ROOT" \
    "$(cat "$BENCH_ROOT/worker-prompt.md")" > "$BENCH_ROOT/worker.jsonl" 2>&1 &
WORKER_CHILD_PID="$!"
WORKER_PROCESS_GROUP_ID="$(ps -o pgid= -p "$WORKER_CHILD_PID" | tr -d ' ')"
if wait "$WORKER_CHILD_PID"; then
    CODE=0
else
    CODE="$?"
fi
WORKER_CHILD_PID=""

END_EPOCH="$(date -u +%s)"
END="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
ELAPSED="$((END_EPOCH - START_EPOCH))"

PROCESS_AUDIT="unavailable"
if [ "$(cat "$PROCESS_AUDIT_STATE")" = "available" ] && [ -n "$WORKER_PROCESS_GROUP_ID" ]; then
    ps -eo pid=,ppid=,pgid=,comm=,args= |
        awk -v group="$WORKER_PROCESS_GROUP_ID" -v pattern="$PROCESS_PATTERN" '$3 == group && tolower($0) ~ pattern' |
        sort > "$CASE_ROOT/process-after.txt" || true
    cp "$CASE_ROOT/process-after.txt" "$CASE_ROOT/process-new.txt"
    if [ -s "$CASE_ROOT/process-new.txt" ]; then
        PROCESS_AUDIT="fail"
    else
        PROCESS_AUDIT="pass"
    fi
else
    cp "$PROCESS_AUDIT_STATE" "$CASE_ROOT/process-after.txt"
fi
cat > "$BENCH_ROOT/process-audit.txt" <<AUDIT
Process audit: $PROCESS_AUDIT
The worker ran in process group $WORKER_PROCESS_GROUP_ID.
The audit checks only matching browser/server/driver processes still belonging
to that worker-owned process group after completion; unrelated host processes
and other parallel workers are excluded.
AUDIT

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
PLAN_DIR="$BENCH_ROOT/$PLAN_NAME"
if [ ! -d "$PLAN_DIR" ] && [ -d "$BENCH_ROOT/.plans/$PLAN_NAME" ]; then
    PLAN_DIR="$BENCH_ROOT/.plans/$PLAN_NAME"
fi
PLAN_FOUND=0
if [ -d "$PLAN_DIR" ]; then
    PLAN_FOUND=1
fi

if [ -x "$SRC_ROOT/planning/scripts/validate-plan.sh" ] && [ "$PLAN_FOUND" -eq 1 ]; then
    if "$SRC_ROOT/planning/scripts/validate-plan.sh" "$PLAN_DIR" > "$BENCH_ROOT/harness-validation.txt" 2>&1; then
        VALIDATION="pass"
    else
        VALIDATION="fail"
    fi
fi

STRUCTURAL_VALIDATION="pass"
STRUCTURAL_REPORT="$BENCH_ROOT/harness-structural-validation.txt"
{
    echo "Structural validation for $PLAN_NAME"
    echo
    require_file() {
        local label="$1"
        local path="$2"
        if [ -f "$path" ]; then
            echo "PASS: $label ($path)"
        else
            echo "FAIL: $label ($path)"
            STRUCTURAL_VALIDATION="fail"
        fi
    }
    require_any_file() {
        local label="$1"
        shift
        local path
        for path in "$@"; do
            if [ -f "$path" ]; then
                echo "PASS: $label ($path)"
                return 0
            fi
        done
        echo "FAIL: $label (none of: $*)"
        STRUCTURAL_VALIDATION="fail"
    }
    require_pattern() {
        local label="$1"
        local pattern="$2"
        local match
        match="$(find "$PLAN_DIR" -type f -iname "$pattern" -print -quit 2>/dev/null)"
        if [ -n "$match" ]; then
            echo "PASS: $label ($match)"
        else
            echo "FAIL: $label (pattern $pattern)"
            STRUCTURAL_VALIDATION="fail"
        fi
    }
    require_any_pattern() {
        local label="$1"
        shift
        local pattern match
        for pattern in "$@"; do
            match="$(find "$PLAN_DIR" -type f -iname "$pattern" -print -quit 2>/dev/null)"
            if [ -n "$match" ]; then
                echo "PASS: $label ($match)"
                return 0
            fi
        done
        echo "FAIL: $label (patterns: $*)"
        STRUCTURAL_VALIDATION="fail"
    }
    require_directory_with_files() {
        local label="$1"
        local directory="$2"
        if [ -d "$PLAN_DIR/$directory" ] && find "$PLAN_DIR/$directory" -type f -print -quit 2>/dev/null | grep -q .; then
            echo "PASS: $label ($PLAN_DIR/$directory)"
        else
            echo "FAIL: $label ($PLAN_DIR/$directory)"
            STRUCTURAL_VALIDATION="fail"
        fi
    }

    if [ "$PLAN_FOUND" -eq 1 ]; then
        require_file "plan description" "$PLAN_DIR/plan-description.md"
        require_file "plan progress tracker" "$PLAN_DIR/progress.md"
        require_any_file "validation report" "$PLAN_DIR/validation.md" "$PLAN_DIR/validation-results.md"
        require_any_file "analysis report" "$PLAN_DIR/analysis-report.md" "$PLAN_DIR/analysis.md"
        require_any_pattern "goal" 'goal.md'
        require_any_pattern "work-unit inventory" '*work-unit*' '*atomic-work-unit*'
        require_any_pattern "UI user story" '*ui*user*stor*' '*ui*stor*'
        if find "$PLAN_DIR" -type f -iname '*run*cache*' -print -quit 2>/dev/null | grep -q .; then
            echo "PASS: UI story run/cache (matching run/cache artifact)"
        else
            require_directory_with_files "UI story run/cache" 'ui-story-runs'
        fi
        require_pattern "adversarial review" '*adversarial*review*'
        require_any_pattern "bug register" '*bug*' '*bugs*'
        if find "$PLAN_DIR" -type f -iname '*context*snap*' -print -quit 2>/dev/null | grep -q .; then
            echo "PASS: context snapshot (matching context snapshot artifact)"
        else
            require_directory_with_files "context snapshot" 'context/snapshots'
        fi
        require_any_pattern "testing companion" '*-testing.md' '*testing*.md'
    else
        echo "FAIL: plan directory missing ($PLAN_DIR)"
        STRUCTURAL_VALIDATION="fail"
    fi
    echo
    echo "result=$STRUCTURAL_VALIDATION"
} > "$STRUCTURAL_REPORT"

USAGE_RECORDS="$(sed -n 's/^usage_records=//p' "$BENCH_ROOT/telemetry.txt" | head -1)"
TOTAL_USAGE="$(sed -n 's/^total_usage_tokens=//p' "$BENCH_ROOT/telemetry.txt" | head -1)"
TELEMETRY_SOURCE="$(sed -n 's/^telemetry_source=//p' "$BENCH_ROOT/telemetry.txt" | head -1)"
TELEMETRY_STATUS="$(sed -n 's/^telemetry_status=//p' "$BENCH_ROOT/telemetry.txt" | head -1)"
HTML_COUNT="$(find "$BENCH_ROOT" -type f \( -name '*.html' -o -name '*.htm' \) | wc -l | tr -d ' ')"
GOALS="$(find "$PLAN_DIR" -type f -name 'goal.md' 2>/dev/null | wc -l | tr -d ' ')"
WORK_UNIT_INVENTORY="$PLAN_DIR/work-unit-inventory.md"
if [ ! -f "$WORK_UNIT_INVENTORY" ]; then
    WORK_UNIT_INVENTORY="$(find "$PLAN_DIR" -type f -name 'work-unit-inventory.md' -print -quit 2>/dev/null)"
fi
if [ -n "$WORK_UNIT_INVENTORY" ] && [ -f "$WORK_UNIT_INVENTORY" ]; then
    WORK_UNITS="$(awk -F'|' '$2 ~ /^[[:space:]]*(WU-[0-9]+|W[0-9]+)[[:space:]]*$/ {count++} END {print count + 0}' "$WORK_UNIT_INVENTORY")"
else
    WORK_UNITS="unavailable"
fi

STATUS="accepted"
if [ "$CODE" -ne 0 ] || [ "$PLAN_FOUND" -ne 1 ] || [ "$HTML_COUNT" != 0 ] || [ "$VALIDATION" = "fail" ] || [ "$STRUCTURAL_VALIDATION" = "fail" ] || [ "$PROCESS_AUDIT" != "pass" ]; then
    STATUS="tainted"
fi
if [ "$SESSION_ID" = "unavailable" ] || [ "$TELEMETRY_STATUS" != "available" ] || ! [[ "${USAGE_RECORDS:-}" =~ ^[1-9][0-9]*$ ]] || ! [[ "${TOTAL_USAGE:-}" =~ ^[1-9][0-9]*$ ]]; then
    STATUS="tainted"
fi

mkdir -p "$RESULT_DIR"
cp -R "$BENCH_ROOT"/. "$RESULT_DIR"/
cp -R "$SRC_ROOT/planning" "$RESULT_DIR/planning"
cp "$CASE_ROOT/preflight-before-worker.txt" "$RESULT_DIR/preflight-before-worker.txt"

cat > "$RESULT_DIR/evaluation.md" <<EVAL
# Benchmark evaluation $REVISION $RUN_ID

- Status: $STATUS
- Revision: $REVISION ($TAG)
- Benchmarked skill: planning/ from tagged source $TAG
- Isolated directory: $BENCH_ROOT
- Tagged source directory: $SRC_ROOT
- Result archive: $RESULT_DIR
- Plan: $PLAN_NAME
- Worker exit code: $CODE
- Session ID: $SESSION_ID
- Telemetry records: ${USAGE_RECORDS:-unavailable}
- Total usage tokens: ${TOTAL_USAGE:-unavailable}
- Telemetry source: ${TELEMETRY_SOURCE:-unavailable}
- Telemetry status: ${TELEMETRY_STATUS:-unavailable}
- Start: $START
- End: $END
- Elapsed seconds: $ELAPSED
- Work-unit count: $WORK_UNITS
- Goal count: $GOALS
- Validation result: $VALIDATION
- Structural validation result: $STRUCTURAL_VALIDATION
- Process audit result: $PROCESS_AUDIT
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
