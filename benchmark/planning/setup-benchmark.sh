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

# Resolve the active benchmark agent at setup time and persist it into the
# generated case so start-worker.sh and the copied telemetry/session-id scripts
# honour the active agent when the case runs later. BENCHMARK_AGENT overrides
# (must name an installed driver); unset resolves to the codex default.
RUNTIME_DIR="$SCRIPT_DIR/runtime"
source "$RUNTIME_DIR/agent-env.sh"
if ! resolve_active_agent "$RUNTIME_DIR"; then
    echo "setup-benchmark.sh: could not resolve a benchmark agent (BENCHMARK_AGENT=${BENCHMARK_AGENT:-unset})" >&2
    exit 64
fi
BENCHMARK_AGENT="$AGENT_DRIVER"
export BENCHMARK_AGENT
# Load the shared runtime (defines benchmark_result_parent, persona helpers,
# launcher) early so the result-path and prompt logic below can use them.
source "$REPO_ROOT/benchmark/planning/runtime/lib-agent.sh"

TAG="$1"
TEST_BASE_DIR="$2"
RUN_NAME="$3"
RUN_ID="${4:-$(date -u +%Y%m%dT%H%M%SZ)-$RUN_NAME}"
REVISION="${TAG#v}"
CASE_ROOT="$TEST_BASE_DIR/$REVISION"
SRC_ROOT="$CASE_ROOT/source"
BENCH_ROOT="$CASE_ROOT/workspace"
PLAN_NAME="basic-test-proof-${REVISION}-${RUN_ID}-isolated-plan"
# Results live under benchmark/results/<agent>/<revision-parent>/<run-id>/<revision>/
# where <revision-parent> is the tag (or current/<latest-tag> for `current`).
# benchmark_result_parent comes from the sourced runtime lib-agent.sh.
RESULTS_ROOT="$REPO_ROOT/benchmark/results/$AGENT_DRIVER/$(benchmark_result_parent "$TAG")"
RESULT_DIR="$RESULTS_ROOT/$RUN_ID/$REVISION"
STAGING_RESULT_DIR="$REPO_ROOT/benchmark/results/$AGENT_DRIVER/.staging/$RUN_ID/$REVISION"
PROTOCOL_ID="reviewer-optimization-1.4.2"
COHORT="1.4.2"
# Capsule/scratch root lives under the scoped planning-agent temp dir so the
# agent's granted read/write/execute on that dir covers all benchmark scratch.
CAPSULE_BASE="${PLANNING_AGENT_TMPDIR:-${TMPDIR:-/tmp}/planning-agent}/ai-skills-capsules"
CAPSULE_ROOT="$CAPSULE_BASE/$RUN_ID/$REVISION/worker"
WORKER_WORKSPACE="$BENCH_ROOT"
BLINDED_ORACLE_SPEC="${BLINDED_ORACLE_SPEC:-}"
if [ -n "$BLINDED_ORACLE_SPEC" ] && [ ! -f "$BLINDED_ORACLE_SPEC" ]; then
    echo "Blinded oracle defect specification not found: $BLINDED_ORACLE_SPEC" >&2
    exit 66
fi

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

copy_workspace_for_publication() {
    local source_root="$1" target_root="$2"
    mkdir -p "$target_root"
    tar -C "$source_root" \
        --exclude='.env' \
        --exclude='.env.tmp.*' \
        --exclude='*/.env' \
        --exclude='*/.env.tmp.*' \
        -cf - . | tar -C "$target_root" -xf -
}

if [ "$TAG" != current ] && ! git -C "$REPO_ROOT" rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
    echo "Tag not found: $TAG" >&2
    exit 66
fi

if [ -e "$CASE_ROOT" ]; then
    echo "Benchmark case already exists: $CASE_ROOT" >&2
    exit 73
fi

mkdir -p "$SRC_ROOT" "$BENCH_ROOT" "$STAGING_RESULT_DIR"
if [ "$TAG" = current ]; then
    tar -C "$REPO_ROOT" \
        --exclude='.git' --exclude='.plans' --exclude='benchmark/results' \
        --exclude='benchmark/.git' -cf - . | tar -x -C "$SRC_ROOT"
else
    git -C "$REPO_ROOT" archive "$TAG" | tar -x -C "$SRC_ROOT"
fi

COMPATIBILITY_REPORT="$BENCH_ROOT/benchmark-compatibility.txt"
printf 'No compatibility fallback is permitted for %s.\n' "$TAG" > "$COMPATIBILITY_REPORT"
for CONTEXT_TEST in \
    "$SRC_ROOT/planning/tests/test-plan-context.sh" \
    "$SRC_ROOT/planning/tests/test-plan-context-deferred-boundary.sh"; do
    if [ ! -f "$CONTEXT_TEST" ]; then
        continue
    fi

    if grep -Fq '/home/tschallacka/.codex/skills/planning/plans/planning-context-cache' "$CONTEXT_TEST"; then
        printf 'legacy fixture fallback detected: %s\n' "$CONTEXT_TEST" >> "$COMPATIBILITY_REPORT"
        printf 'legacy fixture fallback detected; refusing compatibility patch: %s\n' "$CONTEXT_TEST" >&2
        exit 78
    fi
done

if [ ! -f "$SRC_ROOT/basic-test-proof-plan.md" ]; then
    cp "$SCRIPT_DIR/task-spec.md" "$SRC_ROOT/basic-test-proof-plan.md"
fi

cp "$SCRIPT_DIR/benchmark-test.md" "$BENCH_ROOT/benchmark-test.md"
cp "$SCRIPT_DIR/task-spec.md" "$BENCH_ROOT/task-spec.md"
cp "$SCRIPT_DIR/telemetry.sh" "$CASE_ROOT/telemetry.sh"
cp "$SCRIPT_DIR/session-id-from-jsonl.sh" "$CASE_ROOT/session-id-from-jsonl.sh"
mkdir -p "$CAPSULE_ROOT/planning/references"
mkdir -p "$CAPSULE_ROOT/planning/scripts"
cp "$SCRIPT_DIR/task-spec.md" "$CAPSULE_ROOT/task-spec.md"
cp "$SRC_ROOT/planning/SKILL.md" "$CAPSULE_ROOT/planning/SKILL.md"
cp -R "$SRC_ROOT/planning/scripts/." "$CAPSULE_ROOT/planning/scripts/"
if [ -f "$SRC_ROOT/planning/REVIEWER.md" ]; then
    cp "$SRC_ROOT/planning/REVIEWER.md" "$CAPSULE_ROOT/planning/REVIEWER.md"
fi
if [ -d "$SRC_ROOT/planning/references" ]; then
    cp "$SRC_ROOT/planning/references/ui-user-story-validation.md" "$CAPSULE_ROOT/planning/references/ui-user-story-validation.md"
    cp "$SRC_ROOT/planning/references/plan-read-contract.md" "$CAPSULE_ROOT/planning/references/plan-read-contract.md"
    cp "$SRC_ROOT/planning/references/comment-discipline-contract.md" "$CAPSULE_ROOT/planning/references/comment-discipline-contract.md"
fi
if [ -f "$SRC_ROOT/basic-test-proof-plan.md" ]; then
    cp "$SRC_ROOT/basic-test-proof-plan.md" "$CAPSULE_ROOT/basic-test-proof-plan.md"
fi
CAPSULE_MANIFEST="$CAPSULE_ROOT/worker-manifest.json"
{
    printf '{"schema_version":"1.4.2","root":"%s","entries":[' "$CAPSULE_ROOT"
    first=1
    while IFS= read -r file; do
        rel="${file#"$CAPSULE_ROOT"/}"
        hash="$(sha256sum "$file" | awk '{print $1}')"
        [ "$first" -eq 1 ] || printf ','
        printf '{"path":"%s","sha256":"%s","role":"input"}' "$rel" "$hash"
        first=0
    done < <(find "$CAPSULE_ROOT" -type f ! -name worker-manifest.json -print | sort)
    printf '],"source_hash":"%s"}\n' "$(sha256sum "$CAPSULE_ROOT/planning/SKILL.md" | awk '{print $1}')"
} > "$CAPSULE_MANIFEST"
printf '%s\n' '#!/usr/bin/env bash' 'set -euo pipefail' \
    'exec "$@"' > "$CAPSULE_ROOT/plan-context-wrapper.sh"
chmod +x "$CAPSULE_ROOT/plan-context-wrapper.sh"
render_template "$SCRIPT_DIR/worker-prompt.md" "$BENCH_ROOT/worker-prompt.md"
sed -i "s#$(escape_sed_replacement "$SRC_ROOT")#$(escape_sed_replacement "$CAPSULE_ROOT")#g" "$BENCH_ROOT/worker-prompt.md"

cat > "$CASE_ROOT/benchmark-env.sh" <<EOF
#!/usr/bin/env bash
export REPO_ROOT=$(printf '%q' "$REPO_ROOT")
export BENCHMARK_AGENT=$(printf '%q' "$BENCHMARK_AGENT")
export TAG=$(printf '%q' "$TAG")
export REVISION=$(printf '%q' "$REVISION")
export RUN_ID=$(printf '%q' "$RUN_ID")
export CASE_ROOT=$(printf '%q' "$CASE_ROOT")
export SRC_ROOT=$(printf '%q' "$SRC_ROOT")
export BENCH_ROOT=$(printf '%q' "$BENCH_ROOT")
export PLAN_NAME=$(printf '%q' "$PLAN_NAME")
export RESULT_DIR=$(printf '%q' "$RESULT_DIR")
export STAGING_RESULT_DIR=$(printf '%q' "$STAGING_RESULT_DIR")
export PROTOCOL_ID=$(printf '%q' "$PROTOCOL_ID")
export COHORT=$(printf '%q' "$COHORT")
export WORKER_CAPSULE=$(printf '%q' "$CAPSULE_ROOT")
export CAPSULE_ROOT=$(printf '%q' "$CAPSULE_ROOT")
export CAPSULE_BASE=$(printf '%q' "$CAPSULE_BASE")
export WORKER_WORKSPACE=$(printf '%q' "$WORKER_WORKSPACE")
export REVIEW_MODE=$(printf '%q' "${REVIEW_MODE:-fresh-review}")
export MAX_VERIFICATION_PASSES=$(printf '%q' "${MAX_VERIFICATION_PASSES:-3}")
export MAX_REVIEW_CYCLES=$(printf '%q' "${MAX_REVIEW_CYCLES:-3}")
export BLINDED_ORACLE_SPEC=$(printf '%q' "$BLINDED_ORACLE_SPEC")
# Test-only reviewer seam. These values are inert when unset and are accepted
# only by an isolated harness; the generated adapter still owns authority,
# envelope, semantic, redaction, and fail-closed validation.
export REVIEWER_COMMAND=$(printf '%q' "${REVIEWER_COMMAND:-}")
export REVIEWER_SESSION_ID=$(printf '%q' "${REVIEWER_SESSION_ID:-}")
export REVIEWER_CAPSULE_ID=$(printf '%q' "${REVIEWER_CAPSULE_ID:-}")
export REVIEWER_MODE=$(printf '%q' "${REVIEWER_MODE:-}")
export REVIEWER_APPROVED_AT=$(printf '%q' "${REVIEWER_APPROVED_AT:-}")
export SEMANTIC_THRESHOLD=$(printf '%q' "${SEMANTIC_THRESHOLD:-}")
export INDEPENDENT_THRESHOLD=$(printf '%q' "${INDEPENDENT_THRESHOLD:-}")
EOF

cat > "$CASE_ROOT/start-worker.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/benchmark-env.sh"

# Resolve the active agent driver through the shared runtime. REPO_ROOT is
# exported by benchmark-env.sh; lib-agent.sh sources the resolver and driver
# and exports the launcher (setsid/timeout/mode) and argv builders.
source "$REPO_ROOT/benchmark/planning/runtime/lib-agent.sh"

copy_workspace_for_publication() {
    local source_root="$1" target_root="$2"
    mkdir -p "$target_root"
    tar -C "$source_root" \
        --exclude='.env' \
        --exclude='.env.tmp.*' \
        --exclude='*/.env' \
        --exclude='*/.env.tmp.*' \
        -cf - . | tar -C "$target_root" -xf -
}

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

persona_bootstrap worker || exit 64
persona_bootstrap_prompt "$BENCH_ROOT/worker-prompt.md" worker "$REPO_ROOT/planning/roles/VOICES.md" || exit 64
agent_argv_worker "$BENCH_ROOT" "$WORKER_CAPSULE" "$BENCH_ROOT/worker-prompt.md"
launch_agent setsid "${WORKER_TIMEOUT:-45m}" "$BENCH_ROOT/worker.jsonl"
WORKER_CHILD_PID="$AGENT_PID"
WORKER_PROCESS_GROUP_ID="$AGENT_PGID"
wait_agent
CODE="$AGENT_EXIT"
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

ORACLE_STATUS="not-configured"
ORACLE_TARGET_ROOT=""
ORACLE_PRIVATE_ROOT=""
ORACLE_PLAN_DIR="$PLAN_DIR"
if [ -n "${BLINDED_ORACLE_SPEC:-}" ] && [ "$PLAN_FOUND" -eq 1 ]; then
    ORACLE_STATUS="seeded"
    ORACLE_TARGET_ROOT="$CASE_ROOT/oracle-target"
    ORACLE_PRIVATE_ROOT="/tmp/ai-skills-oracle-private/$RUN_ID/$REVISION"
    mkdir -p "$(dirname "$ORACLE_TARGET_ROOT")" "$(dirname "$ORACLE_PRIVATE_ROOT")"
    if "$REPO_ROOT/benchmark/planning/seed-blinded-defects.sh" \
        "$PLAN_DIR" "$ORACLE_TARGET_ROOT" "$ORACLE_PRIVATE_ROOT" "$BLINDED_ORACLE_SPEC" \
        > "$BENCH_ROOT/oracle-seed.txt" 2>&1; then
        ORACLE_PLAN_DIR="$ORACLE_TARGET_ROOT"
    else
        ORACLE_STATUS="rejected"
        printf '{"schema_version":"1.4.2-blinded-oracle","status":"rejected","reason":"Blinded defect seeding failed; see oracle-seed.txt"}\n' > "$BENCH_ROOT/oracle-rejection.json"
        ORACLE_TARGET_ROOT=""
        ORACLE_PRIVATE_ROOT=""
    fi
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

REVIEWER_LIFECYCLE="$BENCH_ROOT/reviewer-lifecycle.jsonl"
: > "$REVIEWER_LIFECYCLE"
# Worker-internal review subagents are recorded separately from harness-owned
# Reviewer A/B sessions. This preserves the complete event trail without
# falsely treating planning subagents as protocol lifecycle participants.
python3 - "$BENCH_ROOT/worker.jsonl" "$REVIEWER_LIFECYCLE" "$START" "$END" <<'PY'
import json
import sys

worker_log, lifecycle, start, end = sys.argv[1:]
events = []
try:
    with open(worker_log, encoding="utf-8") as handle:
        for line in handle:
            try:
                outer = json.loads(line)
                item = outer.get("item", {})
                tool = item.get("tool")
                if tool not in {"spawn_agent", "close_agent"}:
                    continue
                for session_id in item.get("receiver_thread_ids") or []:
                    events.append({
                        "event_id": f"worker-internal-{tool}-{session_id}",
                        "actor": "worker-subagent",
                        "session_id": session_id,
                        "event_type": "observed-launch" if tool == "spawn_agent" else "observed-termination",
                        "protocol_role": "worker-internal",
                        "cycle": 0,
                        "verification_pass": 0,
                        "review_mode": "not-applicable",
                        "independence": None,
                        "timestamp": start if tool == "spawn_agent" else end,
                    })
            except json.JSONDecodeError:
                continue
except FileNotFoundError:
    pass
with open(lifecycle, "a", encoding="utf-8") as handle:
    for event in events:
        handle.write(json.dumps(event, sort_keys=True) + "\n")
PY
# Reviewer-evidence contract note (documentation only): a finding's "path" and
# "location" fields place its cited evidence using single-file, file-and-section,
# or prose/line location forms. The approval schema validator below enforces the
# structural string requirements; the independent oracle judges which location
# form is correct. This note does not alter benchmark behavior.
approval_schema_validator() {
    local approval_path="$1" report_path="$2"
    # Reviewer-evidence contract (documentation only - no behavior change).
    # Each finding's "path"/"location" pair records where the cited evidence
    # lives, and one of the following location forms is accepted:
    #   (1) single-file path        - path names the file, location is empty/absent
    #   (2) file-and-section        - path names the file, location names a
    #                                section/heading within that file
    #   (3) prose/line location     - path names the file, location cites the
    #                                prose line(s) or line range within it
    # The validator enforces that "path"/"location" are non-empty string fields
    # (below) but does not itself judge which location form is correct; that is
    # the independent oracle's semantic role.
    python3 - "$approval_path" "$report_path" <<'PY'
import datetime as dt
import json
import pathlib
import sys

approval_path, report_path = map(pathlib.Path, sys.argv[1:])
required_strings = ("reviewer_session_id", "capsule_id", "mode", "approved_at", "capsule_manifest_sha256")
finding_strings = ("finding_id", "path", "location", "summary", "observed_contradiction", "impact", "evidence", "required_correction")
reasons = []
try:
    approval = json.loads(approval_path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError):
    approval = None
    reasons.append("APPROVAL_JSON_INVALID")

if not isinstance(approval, dict):
    reasons.append("APPROVAL_WRONG_TYPE")
else:
    for field in required_strings:
        value = approval.get(field)
        if not isinstance(value, str):
            reasons.append(f"APPROVAL_{field.upper()}_WRONG_TYPE")
        elif not value.strip():
            reasons.append(f"APPROVAL_{field.upper()}_EMPTY")
    if approval.get("mode") not in ("fresh-review", "iterative"):
        reasons.append("APPROVAL_MODE_INVALID")
    if isinstance(approval.get("approved_at"), str):
        try:
            dt.datetime.fromisoformat(approval["approved_at"].replace("Z", "+00:00"))
        except ValueError:
            reasons.append("APPROVAL_TIMESTAMP_INVALID")
    if not isinstance(approval.get("overall_plan_approval"), bool):
        reasons.append("APPROVAL_DECISION_WRONG_TYPE")
    finding_ids = set()
    for field in ("approved_findings", "rejected_findings"):
        values = approval.get(field)
        if not isinstance(values, list):
            reasons.append(f"APPROVAL_{field.upper()}_WRONG_TYPE")
            continue
        for index, finding in enumerate(values):
            prefix = f"{field.upper()}_{index}"
            if not isinstance(finding, dict):
                reasons.append(f"{prefix}_WRONG_TYPE")
                continue
            for required in finding_strings:
                value = finding.get(required)
                if not isinstance(value, str):
                    reasons.append(f"{prefix}_{required.upper()}_WRONG_TYPE")
                elif not value.strip():
                    reasons.append(f"{prefix}_{required.upper()}_EMPTY")
            if not isinstance(finding.get("independent"), bool):
                reasons.append(f"{prefix}_INDEPENDENT_WRONG_TYPE")
            finding_id = finding.get("finding_id")
            if isinstance(finding_id, str) and finding_id:
                if finding_id in finding_ids:
                    reasons.append("APPROVAL_DUPLICATE_FINDING_ID")
                finding_ids.add(finding_id)
            if "ambiguous" in finding and not isinstance(finding["ambiguous"], bool):
                reasons.append(f"{prefix}_AMBIGUOUS_WRONG_TYPE")

report = {
    "schema_version": "1.4.2-reviewer-approval",
    "schema_status": "valid" if not reasons else "malformed",
    "schema_reasons": sorted(set(reasons)),
    "approval_path": str(approval_path),
}
report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
raise SystemExit(0 if not reasons else 65)
PY
}

select_reviewer_b_approval() {
    local reviewer_root="$1" lifecycle_path="$2" selection_path="$3"
    python3 - "$reviewer_root" "$lifecycle_path" "$selection_path" <<'PY'
import json
import pathlib
import sys

reviewer_root, lifecycle_path, selection_path = map(pathlib.Path, sys.argv[1:])
roles_by_approval = {}
try:
    lifecycle = [json.loads(line) for line in lifecycle_path.read_text(encoding="utf-8").splitlines() if line.strip()]
except (OSError, json.JSONDecodeError):
    lifecycle = []
for event in lifecycle:
    path = event.get("approval_path")
    role = event.get("protocol_role")
    if isinstance(path, str) and role in ("reviewer-a", "reviewer-b"):
        roles_by_approval[path] = role

candidates = []
unauthorized = []
paths = sorted(reviewer_root.glob("*/plan/approval.json")) if reviewer_root.is_dir() else []
for path in paths:
    path_string = str(path)
    role = roles_by_approval.get(path_string)
    if role is None:
        role = "unknown"
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
        decision = value.get("overall_plan_approval") if isinstance(value, dict) and isinstance(value.get("overall_plan_approval"), bool) else None
    except (OSError, json.JSONDecodeError):
        decision = None
    record = {"path": path_string, "role": role, "overall_plan_approval": decision}
    if role == "reviewer-b":
        candidates.append(record)
    elif role == "reviewer-a":
        unauthorized.append(record)

reasons = []
if not candidates:
    reasons.append("APPROVAL_MISSING")
elif len(candidates) > 1:
    decisions = {item["overall_plan_approval"] for item in candidates if item["overall_plan_approval"] is not None}
    reasons.append("APPROVAL_CONFLICT" if len(decisions) > 1 else "APPROVAL_DUPLICATE")
selected = candidates[0]["path"] if len(candidates) == 1 else None
if unauthorized:
    reasons.append("UNAUTHORIZED_REVIEWER_APPROVAL")
payload = {
    "schema_version": "1.4.2-reviewer-authority",
    "authority": "reviewer-b",
    "candidates": candidates,
    "unauthorized_approvals": unauthorized,
    "selected_approval_path": selected,
    "selection_reasons": sorted(set(reasons)),
}
selection_path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
}

reviewer_b_session_binding() {
    local selection_path="$1" lifecycle_path="$2" current_mode="$3" binding_path="$4"
    python3 - "$selection_path" "$lifecycle_path" "$current_mode" "$binding_path" <<'PY'
import datetime as dt
import hashlib
import json
import pathlib
import sys

selection_path = pathlib.Path(sys.argv[1])
lifecycle_path = pathlib.Path(sys.argv[2])
current_mode = sys.argv[3]
binding_path = pathlib.Path(sys.argv[4])
selection = json.loads(selection_path.read_text(encoding="utf-8"))
selected = selection.get("selected_approval_path")
result = {"schema_version": "1.4.2-reviewer-binding", "binding_status": "rejected", "reason_codes": []}
if not isinstance(selected, str) or not selected:
    result["reason_codes"] = selection.get("selection_reasons") or ["APPROVAL_MISSING"]
    binding_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    raise SystemExit(65)

approval_path = pathlib.Path(selected)
try:
    approval = json.loads(approval_path.read_text(encoding="utf-8"))
    events = [json.loads(line) for line in lifecycle_path.read_text(encoding="utf-8").splitlines() if line.strip()]
except (OSError, json.JSONDecodeError):
    result["reason_codes"] = ["REVIEWER_BINDING_EVIDENCE_INVALID"]
    binding_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    raise SystemExit(65)

handoffs = [event for event in events if event.get("event_type") == "handoff" and event.get("protocol_role") == "reviewer-b" and event.get("approval_path") == selected]
if len(handoffs) != 1:
    result["reason_codes"] = ["REVIEWER_BINDING_MISSING_EVENT" if not handoffs else "REVIEWER_BINDING_DUPLICATE"]
    binding_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    raise SystemExit(65)
handoff = handoffs[0]
launches = [event for event in events if event.get("event_type") == "launch" and event.get("protocol_role") == "reviewer-b" and event.get("session_id") == handoff.get("session_id")]
reasons = []
if len(launches) != 1:
    reasons.append("REVIEWER_BINDING_MISSING_EVENT")
launch = launches[0] if len(launches) == 1 else {}
if approval.get("reviewer_session_id") != handoff.get("session_id"):
    reasons.append("REVIEWER_BINDING_SESSION_MISMATCH")
if approval.get("capsule_id") != handoff.get("capsule_id"):
    reasons.append("REVIEWER_BINDING_CAPSULE_MISMATCH")
if approval.get("mode") != current_mode or handoff.get("review_mode") != current_mode:
    reasons.append("REVIEWER_BINDING_MODE_MISMATCH")
manifest_path = approval_path.parent.parent / "capsule-manifest.json"
actual_manifest_hash = hashlib.sha256(manifest_path.read_bytes()).hexdigest() if manifest_path.is_file() else None
if not actual_manifest_hash or approval.get("capsule_manifest_sha256") != actual_manifest_hash or handoff.get("capsule_manifest_sha256") != actual_manifest_hash:
    reasons.append("REVIEWER_BINDING_MANIFEST_MISMATCH")
if approval_path != approval_path.parent.parent / "plan" / "approval.json":
    reasons.append("REVIEWER_BINDING_CROSS_CAPSULE")
if len(list(approval_path.parent.parent.rglob("approval.json"))) != 1:
    reasons.append("REVIEWER_BINDING_DUPLICATE_APPROVAL")
try:
    approved_at = dt.datetime.fromisoformat(str(approval.get("approved_at", "")).replace("Z", "+00:00"))
    launched_at = dt.datetime.fromisoformat(str(launch.get("timestamp", "")).replace("Z", "+00:00"))
    handed_off_at = dt.datetime.fromisoformat(str(handoff.get("timestamp", "")).replace("Z", "+00:00"))
    if approved_at < launched_at or approved_at > handed_off_at:
        reasons.append("REVIEWER_BINDING_STALE")
except (TypeError, ValueError):
    reasons.append("REVIEWER_BINDING_STALE")

if reasons:
    result["reason_codes"] = sorted(set(reasons))
else:
    result.update({
        "binding_status": "passed",
        "approval_path": selected,
        "reviewer_role": "reviewer-b",
        "reviewer_session_id": handoff["session_id"],
        "capsule_id": handoff["capsule_id"],
        "capsule_manifest_sha256": actual_manifest_hash,
        "mode": current_mode,
        "approved_at": approval["approved_at"],
        "overall_plan_approval": approval["overall_plan_approval"],
    })
binding_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
raise SystemExit(0 if result["binding_status"] == "passed" else 65)
PY
}

REVIEWER_STATUS="not-run"
REVIEWER_B_TRANSCRIPT=""
run_reviewer() {
    local role="$1" session capsule capsule_id review_mode approved_at workspace prompt output code reviewer_started_at reviewer_ended_at capsule_manifest_sha256 approval=""
    if [ "$role" = B ] && [ -n "${REVIEWER_COMMAND:-}" ]; then
        session="${REVIEWER_SESSION_ID:?REVIEWER_SESSION_ID is required with REVIEWER_COMMAND}"
        capsule_id="${REVIEWER_CAPSULE_ID:?REVIEWER_CAPSULE_ID is required with REVIEWER_COMMAND}"
        review_mode="${REVIEWER_MODE:-fresh-review}"
        approved_at="${REVIEWER_APPROVED_AT:?REVIEWER_APPROVED_AT is required with REVIEWER_COMMAND}"
    else
        session="${RUN_ID}-${REVISION}-${role}-$(date -u +%s%N)"
        capsule_id="$session"
        review_mode="${REVIEW_MODE:-fresh-review}"
        approved_at="${REVIEWER_APPROVED_AT:-}"
    fi
    capsule="$CAPSULE_BASE/$RUN_ID/$REVISION/reviewers/$session"
    workspace="/tmp/$RUN_ID/$REVISION/reviewers/$session/workspace"
    reviewer_started_at="${approved_at:-$(date -u +%Y-%m-%dT%H:%M:%SZ)}"
    mkdir -p "$capsule" "$workspace"
    cp -R "$ORACLE_PLAN_DIR" "$capsule/plan"
    cp "$CAPSULE_ROOT/task-spec.md" "$capsule/task-spec.md"
    cp "$CAPSULE_ROOT/planning/SKILL.md" "$capsule/SKILL.md"
    [ -f "$CAPSULE_ROOT/planning/REVIEWER.md" ] && cp "$CAPSULE_ROOT/planning/REVIEWER.md" "$capsule/REVIEWER.md" || true
    prompt="$capsule/reviewer-prompt.md"
    output="$workspace/reviewer.jsonl"
    rp_id="$(persona_id_for "reviewer-$(printf '%s' "$role" | tr '[:upper:]' '[:lower:]')")"
    rp_voice="$(persona_voice "$rp_id" "$REPO_ROOT/planning/roles/VOICES.md" 2>/dev/null)"
    python3 - "$capsule" "$session" "$capsule_id" "$review_mode" "$approved_at" <<'PY'
import hashlib
import json
import pathlib
import sys

capsule = pathlib.Path(sys.argv[1])
session, capsule_id, review_mode, approved_at = sys.argv[2:]
entries = []
for path in sorted(p for p in capsule.rglob("*") if p.is_file() and p.name != "capsule-manifest.json"):
    entries.append({
        "path": str(path.relative_to(capsule)),
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "role": "input",
    })
manifest = {
    "schema_version": "1.4.2",
    "capsule_id": capsule_id,
    "reviewer_session_id": session,
    "mode": review_mode,
    "approved_at": approved_at or None,
    "entries": entries,
}
manifest["sha256"] = hashlib.sha256(
    json.dumps(manifest, sort_keys=True, separators=(",", ":")).encode("utf-8")
).hexdigest()
(capsule / "capsule-manifest.json").write_text(
    json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
)
PY
    capsule_manifest_sha256="$(sha256sum "$capsule/capsule-manifest.json" | awk '{print $1}')"
    cat > "$prompt" <<PROMPT
[PERSONA] id=$rp_id ROLE_ID=$rp_id
Voice: $rp_voice

Review the plan at $capsule/plan using only this capsule and workspace.
This is Reviewer $role in protocol 1.4.2. Record stable AR-NN findings and
write concise evidence. Reviewer A may verify only its owned findings and may
not approve the overall plan. Reviewer B must perform the final independent
review and write approval.json with reviewer_session_id, mode,
capsule_id, capsule_manifest_sha256, approved_findings, rejected_findings,
approved_at, and overall_plan_approval. For Reviewer B, use exactly these
identity values: reviewer_session_id=$session, capsule_id=$capsule_id,
mode=$review_mode, and capsule_manifest_sha256=$capsule_manifest_sha256.
Use an RFC3339 approved_at timestamp recorded during this review.
Every item in approved_findings must be a complete object with non-empty string
fields finding_id, path, location, summary, observed_contradiction, impact,
evidence, and required_correction, plus boolean independent. Consolidated
findings may cover multiple defects. ID-only strings, narrative-only evidence,
preclassified true positives, and objects missing any required field are
terminally invalid; the independent oracle assigns semantic classifications.
Do not inspect parent paths, source checkouts, installed skills, or prior
reviewer capsules.
PROMPT
    printf '{"event_id":"%s-start","actor":"reviewer","session_id":"%s","reviewer_session_id":"%s","capsule_id":"%s","capsule_manifest_sha256":"%s","event_type":"launch","protocol_role":"reviewer-%s","cycle":1,"verification_pass":0,"review_mode":"%s","approved_at":"%s","timestamp":"%s"}\n' "$role" "$session" "$session" "$capsule_id" "$capsule_manifest_sha256" "$(printf '%s' "$role" | tr '[:upper:]' '[:lower:]')" "$review_mode" "$approved_at" "$reviewer_started_at" >> "$REVIEWER_LIFECYCLE"
    if [ "$role" = A ]; then
        persona_bootstrap reviewer-a || exit 64
    else
        persona_bootstrap reviewer-b || exit 64
    fi
    if [ -n "${REVIEWER_COMMAND:-}" ]; then
        REVIEWER_SESSION_ID="$session"
        REVIEWER_CAPSULE_ID="$capsule_id"
        REVIEWER_MODE="$review_mode"
        REVIEWER_APPROVED_AT="$approved_at"
        export REVIEWER_SESSION_ID REVIEWER_CAPSULE_ID REVIEWER_MODE REVIEWER_APPROVED_AT
        if [ "$AGENT_DRIVER" = codex ]; then
            agent_argv_reviewer "$workspace" "$capsule" "$prompt" "$REVIEWER_COMMAND"
        else
            # REVIEWER_COMMAND is a codex-shaped test seam. Under a non-codex
            # driver the argv is driver-shaped and would leak invalid flags to
            # REVIEWER_COMMAND, so ignore it and run the real driver.
            echo "reviewer seam ignored for AGENT_DRIVER=$AGENT_DRIVER (REVIEWER_COMMAND is codex-only); running the real driver" >&2
            agent_argv_reviewer "$workspace" "$capsule" "$prompt"
        fi
    else
        agent_argv_reviewer "$workspace" "$capsule" "$prompt"
    fi
    launch_agent setsid "${REVIEWER_TIMEOUT:-20m}" "$output"
    wait_agent
    code="$AGENT_EXIT"
    if [ "$role" = B ]; then
        approval="$capsule/plan/approval.json"
        # Reviewers may write the protocol handoff beside their capsule plan
        # (the natural location for a reviewer-owned artifact) or inside the
        # plan directory. Accept both locations, then archive the canonical
        # copy under the reviewer result directory.
        if [ ! -s "$approval" ] && [ -s "$capsule/approval.json" ]; then
            approval="$capsule/approval.json"
            cp "$approval" "$capsule/plan/approval.json"
        fi
        if [ ! -s "$approval" ]; then
            code=65
        else
            if ! approval_schema_validator "$approval" "$capsule/approval-schema.json"; then
                code=65
            elif [ -z "${BLINDED_ORACLE_SPEC:-}" ] && ! python3 - "$approval" <<'PY'
import json
import sys
with open(sys.argv[1], encoding="utf-8") as handle:
    approval = json.load(handle)
if approval.get("overall_plan_approval") is not True:
    raise SystemExit(65)
PY
            then
                code=65
            fi
        fi
    fi
    reviewer_ended_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf '{"event_id":"%s-end","actor":"reviewer","session_id":"%s","reviewer_session_id":"%s","capsule_id":"%s","capsule_manifest_sha256":"%s","approval_path":"%s","event_type":"handoff","protocol_role":"reviewer-%s","cycle":1,"verification_pass":1,"review_mode":"%s","approved_at":"%s","exit_code":%s,"independence":%s,"timestamp":"%s"}\n' "$role" "$session" "$session" "$capsule_id" "$capsule_manifest_sha256" "$approval" "$(printf '%s' "$role" | tr '[:upper:]' '[:lower:]')" "$review_mode" "$approved_at" "$code" "$([ "$role" = B ] && echo true || echo false)" "$reviewer_ended_at" >> "$REVIEWER_LIFECYCLE"
    if [ "$role" = B ]; then
        REVIEWER_B_TRANSCRIPT="$output"
    fi
    [ "$code" -eq 0 ] || return "$code"
    REVIEWER_STATUS="passed"
}
if [ "$CODE" -eq 0 ] && [ "$PLAN_FOUND" -eq 1 ] && agent_available; then
    if [ "${REVIEW_MODE:-fresh-review}" = iterative ]; then
        run_reviewer A || REVIEWER_STATUS="failed"
    fi
    if [ "$REVIEWER_STATUS" != failed ]; then
        run_reviewer B || REVIEWER_STATUS="failed"
    fi
else
    printf '{"event_id":"reviewer-not-run","actor":"harness","session_id":null,"event_type":"unavailable","cycle":0,"verification_pass":0,"review_mode":"%s","independence":false,"timestamp":"%s"}\n' \
        "${REVIEW_MODE:-fresh-review}" "$END" >> "$REVIEWER_LIFECYCLE"
fi
if [ "$REVIEWER_STATUS" != passed ]; then STATUS="tainted"; fi

REVIEWER_ROOT="$CAPSULE_BASE/$RUN_ID/$REVISION/reviewers"
REVIEWER_SELECTION="$BENCH_ROOT/reviewer-selection.json"
REVIEWER_SCHEMA="$BENCH_ROOT/reviewer-approval-schema.json"
REVIEWER_BINDING="$BENCH_ROOT/reviewer-b-session-binding.json"
select_reviewer_b_approval "$REVIEWER_ROOT" "$REVIEWER_LIFECYCLE" "$REVIEWER_SELECTION"
REVIEWER_B_APPROVAL="$(python3 - "$REVIEWER_SELECTION" <<'PY'
import json
import sys
try:
    value = json.load(open(sys.argv[1], encoding="utf-8"))
except (OSError, json.JSONDecodeError):
    value = {}
print(value.get("selected_approval_path") or "")
PY
)"
if [ -n "$REVIEWER_B_APPROVAL" ] && [ -s "$REVIEWER_B_APPROVAL" ]; then
    approval_schema_validator "$REVIEWER_B_APPROVAL" "$REVIEWER_SCHEMA" || STATUS="tainted"
fi
if ! reviewer_b_session_binding "$REVIEWER_SELECTION" "$REVIEWER_LIFECYCLE" "${REVIEW_MODE:-fresh-review}" "$REVIEWER_BINDING"; then
    STATUS="tainted"
fi

if [ "$ORACLE_STATUS" = seeded ] && [ "$REVIEWER_STATUS" = passed ]; then
    ORACLE_EVIDENCE="$BENCH_ROOT/oracle-terminal-evidence.json"
    ORACLE_REPORT="$BENCH_ROOT/oracle.json"
    if [ "$(python3 - "$REVIEWER_BINDING" <<'PY'
import json
import sys
try:
    value = json.load(open(sys.argv[1], encoding="utf-8"))
except (OSError, json.JSONDecodeError):
    value = {}
print(value.get("binding_status", "rejected"))
PY
)" != passed ]; then
        ORACLE_STATUS="rejected"
        printf '{"schema_version":"1.4.2-blinded-oracle","status":"rejected","reason":"Reviewer B authority or identity binding failed"}\n' > "$BENCH_ROOT/oracle-rejection.json"
    else
        python3 - "$REVIEWER_B_APPROVAL" "${REVIEWER_B_TRANSCRIPT:-$BENCH_ROOT/worker.jsonl}" "$ORACLE_EVIDENCE" <<'PY'
import hashlib
import json
import pathlib
import sys

approval_path, transcript_path, evidence_path = map(pathlib.Path, sys.argv[1:])
approval = json.loads(approval_path.read_text(encoding="utf-8"))
findings = []
values = approval.get("approved_findings", [])
if isinstance(values, list):
    for value in values:
        if isinstance(value, str):
            # Keep ID-only approval evidence visible to the oracle so it can
            # fail closed as malformed instead of silently dropping it.
            findings.append({"finding_id": value, "independent": True})
        elif isinstance(value, dict):
            finding = dict(value)
            if "finding_id" not in finding and "id" in finding:
                finding["finding_id"] = finding.pop("id")
            # Reviewer B is the selected independent reviewer. Preserve an
            # explicit boolean supplied by the reviewer, and add provenance
            # when the external approval object omitted it.
            finding.setdefault("independent", True)
            findings.append(finding)
        else:
            findings.append(value)
evidence = {
    "terminal": True,
    "target_role": "reviewer-b",
    "transcript_sha256": hashlib.sha256(transcript_path.read_bytes()).hexdigest(),
    "findings": findings,
    "evidence_paths": [
        "reviewer-selection.json",
        "reviewer-b-session-binding.json",
        "reviewer-lifecycle.jsonl",
        "reviewers/" + approval_path.parent.parent.name + "/workspace/reviewer.jsonl"
        if approval_path.parent.parent.name else "worker.jsonl",
    ],
}
evidence_path.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
        if ORACLE_ROLE=independent-oracle ORACLE_MODE="${REVIEW_MODE:-fresh-review}" \
            ORACLE_REVISION="$REVISION" ORACLE_RUN_ID="$RUN_ID" \
            "$REPO_ROOT/benchmark/planning/review-oracle.sh" blinded \
            "$ORACLE_PRIVATE_ROOT" "$ORACLE_PRIVATE_ROOT/target-snapshot" "$ORACLE_EVIDENCE" "$ORACLE_REPORT" \
            > "$BENCH_ROOT/oracle-grade.txt" 2>&1; then
            ORACLE_STATUS="accepted"
        else
            ORACLE_STATUS="rejected"
            printf '{"schema_version":"1.4.2-blinded-oracle","status":"rejected","reason":"Independent oracle grading failed"}\n' > "$BENCH_ROOT/oracle-rejection.json"
            rm -f "$ORACLE_REPORT"
        fi
    fi
elif [ -n "${BLINDED_ORACLE_SPEC:-}" ] && [ "$ORACLE_STATUS" != rejected ]; then
    ORACLE_STATUS="rejected"
    printf '{"schema_version":"1.4.2-blinded-oracle","status":"rejected","reason":"Target did not reach terminal reviewer evidence"}\n' > "$BENCH_ROOT/oracle-rejection.json"
fi
if [ "$ORACLE_STATUS" = rejected ]; then
    STATUS="tainted"
fi
PROVENANCE_MATERIAL="$BENCH_ROOT/provenance-material.json"
DEFECTIVE_PLAN_PATH=""
TARGET_SNAPSHOT_PATH=""
if [ -n "$ORACLE_TARGET_ROOT" ] && [ -n "$ORACLE_PRIVATE_ROOT" ]; then
    DEFECTIVE_PLAN_PATH="$ORACLE_PLAN_DIR"
    TARGET_SNAPSHOT_PATH="$ORACLE_PRIVATE_ROOT/target-snapshot"
fi
python3 - "$PROVENANCE_MATERIAL" "$PLAN_DIR" "$DEFECTIVE_PLAN_PATH" "$TARGET_SNAPSHOT_PATH" "$REVIEWER_B_APPROVAL" "${REVIEWER_B_TRANSCRIPT:-$BENCH_ROOT/worker.jsonl}" "$REVIEWER_LIFECYCLE" "$REVIEWER_ROOT" "$REVIEWER_BINDING" "$REVIEWER_SELECTION" <<'PY'
import hashlib
import json
import pathlib
import sys

output, source_plan, defective_plan, target_snapshot, approval, transcript, lifecycle, reviewer_root, binding, selection = sys.argv[1:]
def digest_tree(root):
    if not root or not pathlib.Path(root).is_dir():
        return None, []
    root = pathlib.Path(root)
    entries = []
    for path in sorted(p for p in root.rglob("*") if p.is_file()):
        relative = str(path.relative_to(root))
        entries.append({"path": relative, "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
    encoded = json.dumps(entries, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest(), entries
def digest_file(path):
    candidate = pathlib.Path(path) if path else None
    return hashlib.sha256(candidate.read_bytes()).hexdigest() if candidate and candidate.is_file() else None
def archive_reviewer_path(path):
    if not path:
        return None
    candidate = pathlib.Path(path)
    try:
        relative = candidate.relative_to(pathlib.Path(reviewer_root))
    except ValueError:
        return None
    return str(pathlib.PurePosixPath("reviewers") / relative)

source_hash, source_entries = digest_tree(source_plan)
defective_hash, defective_entries = digest_tree(defective_plan if defective_plan != source_plan else None)
snapshot_hash, snapshot_entries = digest_tree(target_snapshot)
selected = None
try:
    selected = json.loads(pathlib.Path(binding).read_text(encoding="utf-8")).get("approval_path")
except (OSError, json.JSONDecodeError):
    pass
selected = selected or approval
selected_relative = archive_reviewer_path(selected)
capsule_relative = str(pathlib.PurePosixPath(selected_relative).parent.parent / "capsule-manifest.json") if selected_relative else None
payload = {
    "schema_version": "1.4.2-provenance",
    "source_plan": {"sha256": source_hash, "archive_path": str(pathlib.PurePosixPath(source_plan).name) if source_hash else None},
    "defective_plan": {"sha256": defective_hash, "archive_path": "provenance/defective-plan-manifest.json" if defective_hash else None},
    "target_snapshot": {"sha256": snapshot_hash, "archive_path": "provenance/target-snapshot-manifest.json" if snapshot_hash else None},
    "reviewer_approval": {"sha256": digest_file(selected), "archive_path": selected_relative},
    "reviewer_capsule": {"sha256": digest_file(pathlib.Path(reviewer_root) / pathlib.Path(selected).parent.parent.name / "capsule-manifest.json") if selected else None, "archive_path": capsule_relative},
    "transcript": {"sha256": digest_file(transcript), "archive_path": archive_reviewer_path(transcript) or "worker.jsonl"},
    "lifecycle_handoff": {"sha256": digest_file(lifecycle), "archive_path": "reviewer-lifecycle.jsonl"},
    "binding": {"archive_path": "reviewer-b-session-binding.json", "sha256": digest_file(binding)},
    "selection": {"archive_path": "reviewer-selection.json", "sha256": digest_file(selection)},
}
pathlib.Path(output).write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
if defective_entries:
    pathlib.Path(output).parent.joinpath("provenance").mkdir(parents=True, exist_ok=True)
    pathlib.Path(output).parent.joinpath("provenance/defective-plan-manifest.json").write_text(json.dumps({"root_sha256": defective_hash, "entries": defective_entries}, indent=2, sort_keys=True) + "\n", encoding="utf-8")
if snapshot_entries:
    pathlib.Path(output).parent.joinpath("provenance").mkdir(parents=True, exist_ok=True)
    pathlib.Path(output).parent.joinpath("provenance/target-snapshot-manifest.json").write_text(json.dumps({"root_sha256": snapshot_hash, "entries": snapshot_entries}, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
if [ -n "$ORACLE_PRIVATE_ROOT" ]; then
    rm -rf "$ORACLE_PRIVATE_ROOT"
fi

# Approval is terminal review evidence, not an adoption decision. Keep this
# state calculation independent from worker/oracle exit status so a valid
# negative approval remains gradeable while adoption fails closed.
REVIEWER_STATE="$BENCH_ROOT/reviewer-state.json"
export REVIEWER_STATUS ORACLE_STATUS STATUS REVIEWER_STATE
python3 - "$REVIEWER_STATE" "$BENCH_ROOT/oracle.json" "$REVIEWER_SELECTION" "$REVIEWER_SCHEMA" "$REVIEWER_BINDING" "$PROVENANCE_MATERIAL" <<'PY'
import json
import os
import pathlib
import sys

state_path, oracle_path, selection_path, schema_path, binding_path, provenance_path = map(pathlib.Path, sys.argv[1:])
reasons = set()
def read_json(path):
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
        return value if isinstance(value, dict) else {}
    except (OSError, json.JSONDecodeError):
        return {}
selection = read_json(selection_path)
schema = read_json(schema_path)
binding = read_json(binding_path)
provenance = read_json(provenance_path)
reasons.update(selection.get("selection_reasons", []))
if selection.get("unauthorized_approvals"):
    reasons.add("UNAUTHORIZED_REVIEWER_APPROVAL")
selected = selection.get("selected_approval_path")
approval = read_json(pathlib.Path(selected)) if isinstance(selected, str) and selected else {}
if selected and schema.get("schema_status") != "valid":
    reasons.add("APPROVAL_SCHEMA_INVALID")
if selected and binding.get("binding_status") != "passed":
    reasons.update(binding.get("reason_codes", []))
provenance_keys = ("source_plan", "defective_plan", "target_snapshot", "reviewer_approval", "reviewer_capsule", "transcript", "lifecycle_handoff")
if os.environ.get("ORACLE_STATUS") == "accepted" or os.environ.get("ORACLE_STATUS") == "seeded":
    if any(not isinstance(provenance.get(key), dict) or not provenance[key].get("sha256") or not provenance[key].get("archive_path") for key in provenance_keys):
        reasons.add("PROVENANCE_MISSING")
review_completed = os.environ.get("REVIEWER_STATUS") == "passed"
if not review_completed:
    reasons.add("REVIEW_INCOMPLETE")
binding_passed = binding.get("binding_status") == "passed"
plan_approved = bool(binding_passed and approval.get("overall_plan_approval") is True)
if binding_passed and approval.get("overall_plan_approval") is False:
    reasons.update(("APPROVAL_REJECTED", "PLAN_NOT_APPROVED"))
elif not binding_passed:
    reasons.add("PLAN_NOT_APPROVED")

try:
    oracle = json.loads(oracle_path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError):
    oracle = {}
oracle_completed = os.environ.get("ORACLE_STATUS") == "accepted" and bool(oracle)
if not oracle_completed:
    reasons.add("ORACLE_INCOMPLETE")
counts = oracle.get("counts", {}) if isinstance(oracle, dict) else {}
ambiguous = counts.get("ambiguous", oracle.get("ambiguous", 0) if isinstance(oracle, dict) else 0)
if isinstance(ambiguous, int) and ambiguous > 0:
    reasons.add("ORACLE_AMBIGUOUS")
denominators = oracle.get("denominators", {}) if isinstance(oracle, dict) else {}
denominator = denominators.get("seeded") if isinstance(denominators, dict) else None
if not isinstance(denominator, int):
    denominator = oracle.get("seeded_denominator") if isinstance(oracle, dict) else None
semantic_rate = oracle.get("semantic_true_positive_rate", oracle.get("true_positive_rate")) if isinstance(oracle, dict) else None
independent_rate = oracle.get("independent_catch_rate") if isinstance(oracle, dict) else None
try:
    semantic_threshold = float(os.environ["SEMANTIC_THRESHOLD"])
    independent_threshold = float(os.environ["INDEPENDENT_THRESHOLD"])
except (KeyError, TypeError, ValueError):
    semantic_threshold = independent_threshold = None
if not isinstance(denominator, int) or denominator <= 0:
    reasons.add("MISSING_DENOMINATOR")
if semantic_threshold is None or independent_threshold is None:
    reasons.add("MISSING_THRESHOLDS")
if isinstance(semantic_rate, (int, float)) and semantic_threshold is not None and semantic_rate < semantic_threshold:
    reasons.add("SEMANTIC_THRESHOLD_FAILED")
if isinstance(independent_rate, (int, float)) and independent_threshold is not None and independent_rate < independent_threshold:
    reasons.add("INDEPENDENT_THRESHOLD_FAILED")
# Sanitized per-defect projection from the public oracle report. Only the
# neutral ordinal id, public finding ids, failed predicates, and classification
# are carried forward; private defect material is never projected into any
# public artifact.
per_defect = []
if isinstance(oracle.get("per_defect"), list):
    for entry in oracle["per_defect"]:
        if not isinstance(entry, dict):
            continue
        projection = {}
        if isinstance(entry.get("index"), str):
            projection["index"] = entry["index"]
        if isinstance(entry.get("finding_ids"), list):
            projection["finding_ids"] = [fid for fid in entry["finding_ids"] if isinstance(fid, str)]
        if isinstance(entry.get("failed_predicates"), list):
            projection["failed_predicates"] = [p for p in entry["failed_predicates"] if isinstance(p, str)]
        if isinstance(entry.get("classification"), str):
            projection["classification"] = entry["classification"]
        per_defect.append(projection)
if os.environ.get("STATUS") == "tainted":
    reasons.add("TAINTED_RUN")
b_candidates = selection.get("candidates", [])
decisions = [item.get("overall_plan_approval") for item in b_candidates if isinstance(item, dict) and isinstance(item.get("overall_plan_approval"), bool)]
approval_conflict = len(set(decisions)) > 1
adoptable = review_completed and plan_approved and oracle_completed and not reasons
state = {
    "review_completed": review_completed,
    "reviewer_authority": "reviewer-b",
    "plan_approved": plan_approved,
    "oracle_completed": oracle_completed,
    "adoptable": adoptable,
    "approval_conflict": approval_conflict,
    "approval_schema_status": schema.get("schema_status", "not-run"),
    "reviewer_binding_status": binding.get("binding_status", "not-run"),
    "selected_reviewer_session_id": binding.get("reviewer_session_id"),
    "selected_capsule_id": binding.get("capsule_id"),
    "selected_capsule_manifest_sha256": binding.get("capsule_manifest_sha256"),
    "semantic_true_positive_rate": semantic_rate if isinstance(semantic_rate, (int, float)) else None,
    "independent_catch_rate": independent_rate if isinstance(independent_rate, (int, float)) else None,
    "seeded_denominator": denominator if isinstance(denominator, int) else 0,
    "fail_closed_reasons": sorted(reasons),
    "semantic_threshold": semantic_threshold,
    "independent_threshold": independent_threshold,
    "per_defect": per_defect,
    "provenance": provenance,
    "source_plan_sha256": provenance.get("source_plan", {}).get("sha256"),
    "defective_plan_sha256": provenance.get("defective_plan", {}).get("sha256"),
    "target_snapshot_sha256": provenance.get("target_snapshot", {}).get("sha256"),
    "approval_sha256": provenance.get("reviewer_approval", {}).get("sha256"),
    "transcript_sha256": provenance.get("transcript", {}).get("sha256"),
    "lifecycle_handoff_sha256": provenance.get("lifecycle_handoff", {}).get("sha256"),
    "provenance_paths": {key: value.get("archive_path") for key, value in provenance.items() if isinstance(value, dict) and value.get("archive_path")},
}
state_path.write_text(json.dumps(state, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY

# Preserve one machine-readable, provenance-bearing telemetry artifact next to
# the human-readable lookup. Values unavailable from the runner remain null or
# explicitly unavailable; this file never substitutes inferred totals.
export PROTOCOL_ID RUN_ID REVISION SESSION_ID STATUS REVIEW_MODE START END ELAPSED
export CODE VALIDATION STRUCTURAL_VALIDATION PROCESS_AUDIT HTML_COUNT REVIEWER_STATUS
export TELEMETRY_SOURCE TELEMETRY_STATUS TOTAL_USAGE USAGE_RECORDS REVIEWER_LIFECYCLE ORACLE_STATUS REVIEWER_STATE
python3 - <<'PY' > "$BENCH_ROOT/telemetry.json"
import json
import os

status = os.environ["STATUS"]
with open(os.environ["REVIEWER_STATE"], encoding="utf-8") as handle:
    reviewer_state = json.load(handle)
taint = []
def cause(code, layer, path):
    taint.append({"code": code, "layer": layer, "evidence_paths": [path], "event_ids": []})
if os.environ.get("CODE") != "0":
    cause("WORKER_EXIT", "worker", "worker.jsonl")
if os.environ.get("VALIDATION") == "fail":
    cause("VALIDATION_FAILED", "validation", "harness-validation.txt")
if os.environ.get("STRUCTURAL_VALIDATION") == "fail":
    cause("STRUCTURAL_GATE_FAILED", "archive", "harness-structural-validation.txt")
if os.environ.get("PROCESS_AUDIT") != "pass":
    cause("PROCESS_AUDIT_FAILED", "process", "process-audit.txt")
if os.environ.get("HTML_COUNT") != "0":
    cause("FORBIDDEN_ARTIFACT", "artifact", "process-audit.txt")
if os.environ.get("TELEMETRY_STATUS") != "available":
    cause("TELEMETRY_UNAVAILABLE", "telemetry", "telemetry.txt")
if os.environ.get("REVIEWER_STATUS") != "passed":
    cause("REVIEWER_LIFECYCLE_FAILED", "review", "reviewer-lifecycle.jsonl")
if os.environ.get("ORACLE_STATUS") == "rejected":
    cause("BLINDED_ORACLE_FAILED", "oracle", "oracle-rejection.json")
try:
    tokens = int(os.environ["TOTAL_USAGE"])
except (TypeError, ValueError):
    tokens = None
try:
    duration = float(os.environ["ELAPSED"])
except (TypeError, ValueError):
    duration = 0
records_by_session = {}
try:
    with open(os.environ["REVIEWER_LIFECYCLE"], encoding="utf-8") as handle:
        for line in handle:
            event = json.loads(line)
            if event.get("session_id"):
                session_id = event["session_id"]
                record = records_by_session.setdefault(session_id, {
                    "session_id": session_id,
                    "cycle": event.get("cycle", 0),
                    "verification_pass": event.get("verification_pass", 0),
                    "event_count": 0,
                    "token_total": None,
                    "source": "reviewer-lifecycle.jsonl",
                })
                record["event_count"] += 1
                record["verification_pass"] = max(record["verification_pass"], event.get("verification_pass", 0))
except (FileNotFoundError, json.JSONDecodeError):
    pass
payload = {
    "schema_version": "1.4.2",
    "protocol_id": os.environ["PROTOCOL_ID"],
    "run_id": os.environ["RUN_ID"],
    "revision": os.environ["REVISION"],
    "worker_thread_id": os.environ["SESSION_ID"],
    "reviewer_mode": os.environ.get("REVIEW_MODE", "fresh-review"),
    "reviewer_session_id": next(reversed(records_by_session), None),
    "status": status,
    "review_state": reviewer_state,
    "per_defect": reviewer_state.get("per_defect", []),
    "taint_causes": taint,
    "telemetry_source": os.environ["TELEMETRY_SOURCE"],
    "provenance": {
        "telemetry_status": os.environ["TELEMETRY_STATUS"],
        "lookup_artifact": "telemetry.txt",
        "lifecycle_artifact": "reviewer-lifecycle.jsonl",
        "token_total_precision": "exact" if tokens is not None else "unavailable",
        "authority": reviewer_state.get("reviewer_authority"),
        "reviewer": reviewer_state.get("provenance", {}).get("reviewer_approval"),
        "capsule": reviewer_state.get("provenance", {}).get("reviewer_capsule"),
        "seed": reviewer_state.get("provenance", {}).get("target_snapshot"),
        "plan": reviewer_state.get("provenance", {}).get("source_plan"),
        "defective_plan": reviewer_state.get("provenance", {}).get("defective_plan"),
        "transcript": reviewer_state.get("provenance", {}).get("transcript"),
        "lifecycle_handoff": reviewer_state.get("provenance", {}).get("lifecycle_handoff"),
    },
    "phase_records": [{
        "phase": "worker",
        "start": os.environ["START"],
        "end": os.environ["END"],
        "duration_seconds": duration,
        "source": "runner",
        "precision": "exact",
    }],
    "reviewer_records": list(records_by_session.values()),
    "usage": {"total_tokens": tokens, "records": os.environ.get("USAGE_RECORDS", "unavailable")},
}
print(json.dumps(payload, indent=2, sort_keys=True))
PY
if ! "$REPO_ROOT/benchmark/planning/validate-telemetry.sh" \
    "$REPO_ROOT/benchmark/planning/telemetry-schema.json" "$BENCH_ROOT/telemetry.json" \
    > "$BENCH_ROOT/telemetry-validation.txt" 2>&1; then
    STATUS="tainted"
fi

if [ -e "$RESULT_DIR" ]; then
    printf 'archive collision: %s\n' "$RESULT_DIR" > "$STAGING_RESULT_DIR/publication-rejection.txt"
    printf 'collision %s\n' "$RESULT_DIR" >&2
    exit 73
fi
# Archive the deterministic per-defect post-run report alongside oracle.json.
# When no oracle report exists the script emits a deterministic "none" marker.
if [ -f "$BENCH_ROOT/oracle.json" ]; then
    "$REPO_ROOT/benchmark/planning/post-run-report.sh" "$BENCH_ROOT/oracle.json" > "$BENCH_ROOT/post-run-report.txt"
else
    printf 'PER-DEFECT POST-RUN REPORT\nper_defect: none\n' > "$BENCH_ROOT/post-run-report.txt"
fi
copy_workspace_for_publication "$BENCH_ROOT" "$STAGING_RESULT_DIR"
cp -R "$SRC_ROOT/planning" "$STAGING_RESULT_DIR/planning"
cp "$CASE_ROOT/preflight-before-worker.txt" "$STAGING_RESULT_DIR/preflight-before-worker.txt"
if [ -d "$CAPSULE_BASE/$RUN_ID/$REVISION/reviewers" ]; then
    cp -R "$CAPSULE_BASE/$RUN_ID/$REVISION/reviewers" "$STAGING_RESULT_DIR/reviewers"
fi
export PROTOCOL_ID COHORT REVISION TAG RUN_ID WORKER_CAPSULE ORACLE_STATUS REVIEW_MODE
python3 - "$REVIEWER_STATE" "$STAGING_RESULT_DIR/protocol-metadata.json" <<'PY'
import json
import os
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
payload = {
    "protocol_id": os.environ["PROTOCOL_ID"],
    "cohort": os.environ["COHORT"],
    "revision": os.environ["REVISION"],
    "tag": os.environ["TAG"],
    "run_id": os.environ["RUN_ID"],
    "review_mode": os.environ.get("REVIEW_MODE", "fresh-review"),
    "access_control": "worker-capsule",
    "capsule": os.environ["WORKER_CAPSULE"],
    "blinded_oracle": os.environ["ORACLE_STATUS"],
    "reviewer_authority": state["reviewer_authority"],
    "selected_reviewer_session_id": state["selected_reviewer_session_id"],
    "selected_capsule_id": state["selected_capsule_id"],
    "selected_capsule_manifest_sha256": state["selected_capsule_manifest_sha256"],
    "approval_schema_status": state["approval_schema_status"],
    "reviewer_binding_status": state["reviewer_binding_status"],
    "source_plan_sha256": state["source_plan_sha256"],
    "defective_plan_sha256": state["defective_plan_sha256"],
    "target_snapshot_sha256": state["target_snapshot_sha256"],
    "approval_sha256": state["approval_sha256"],
    "transcript_sha256": state["transcript_sha256"],
    "lifecycle_handoff_sha256": state["lifecycle_handoff_sha256"],
    "provenance_paths": state["provenance_paths"],
    "state_schema": {
        "review_completed": "boolean",
        "plan_approved": "boolean",
        "oracle_completed": "boolean",
        "adoptable": "boolean",
        "fail_closed_reasons": "string[]",
        "reviewer_authority": "string",
        "selected_reviewer_session_id": ["string", "null"],
        "selected_capsule_id": ["string", "null"],
        "selected_capsule_manifest_sha256": ["string", "null"],
        "approval_schema_status": "string",
        "reviewer_binding_status": "string",
        "source_plan_sha256": ["string", "null"],
        "defective_plan_sha256": ["string", "null"],
        "target_snapshot_sha256": ["string", "null"],
        "approval_sha256": ["string", "null"],
        "transcript_sha256": ["string", "null"],
        "lifecycle_handoff_sha256": ["string", "null"],
        "provenance_paths": "object",
        "per_defect": "object[]",
    },
    "review_completed": state["review_completed"],
    "plan_approved": state["plan_approved"],
    "oracle_completed": state["oracle_completed"],
    "adoptable": state["adoptable"],
    "fail_closed_reasons": state["fail_closed_reasons"],
    "approval_conflict": state["approval_conflict"],
    "per_defect": state.get("per_defect", []),
    "review_state": state,
    "provenance": state["provenance"],
}
pathlib.Path(sys.argv[2]).write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY

cat > "$STAGING_RESULT_DIR/evaluation.md" <<EVAL
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
- Reviewer lifecycle status: $REVIEWER_STATUS
- Blinded oracle status: $ORACLE_STATUS
EVAL
python3 - "$REVIEWER_STATE" "$STAGING_RESULT_DIR/evaluation.md" <<'PY'
import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
with pathlib.Path(sys.argv[2]).open("a", encoding="utf-8") as handle:
    handle.write("\n## Review and adoption state\n\n")
    for key in ("review_completed", "plan_approved", "oracle_completed", "adoptable"):
        handle.write(f"- {key}: {str(state[key]).lower()}\n")
    handle.write(f"- semantic_true_positive_rate: {state['semantic_true_positive_rate']}\n")
    handle.write(f"- independent_catch_rate: {state['independent_catch_rate']}\n")
    handle.write(f"- seeded_denominator: {state['seeded_denominator']}\n")
    handle.write(f"- approval_conflict: {str(state['approval_conflict']).lower()}\n")
    handle.write(f"- fail_closed_reasons: {json.dumps(state['fail_closed_reasons'])}\n")
PY

mkdir -p "$(dirname "$RESULT_DIR")"
mv "$STAGING_RESULT_DIR" "$RESULT_DIR"

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
