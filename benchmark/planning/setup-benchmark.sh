#!/usr/bin/env bash
# Prepare one tagged planning benchmark case.
#
# Usage:
#   benchmark/planning/setup-benchmark.sh <tag> <testing-base-dir> <name> [run-id]

set -euo pipefail

if [ "$#" -lt 3 ] || [ "$#" -gt 4 ]; then
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

# Resolved at setup time and persisted into the generated case, so the case
# honours the agent it was created for when it runs later.
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
# Platform primitives (sha256, literal in-place replace) shared with the case
# runner, so a GNU-vs-BSD quirk is fixed once. See CODE-STYLE.md § 1.
# shellcheck source=lib-portable.sh
source "$SCRIPT_DIR/lib-portable.sh"

TAG="$1"
TEST_BASE_DIR="$2"
RUN_NAME="$3"
RUN_ID="${4:-$(date -u +%Y%m%dT%H%M%SZ)-$RUN_NAME}"
REVISION="${TAG#v}"
# Case dirs are run-id-suffixed so concurrent or repeated runs never collide;
# each run gets its own scaffolding under the testing base dir.
CASE_ROOT="$TEST_BASE_DIR/$REVISION-$RUN_ID"
SRC_ROOT="$CASE_ROOT/source"
BENCH_ROOT="$CASE_ROOT/workspace"
PLAN_NAME="$(printf '%s' "basic-test-proof-${REVISION}-${RUN_ID}-isolated-plan" | tr '[:upper:]' '[:lower:]')"
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

# Escape a string for use as a sed REPLACEMENT with `/` as the delimiter.
# Backslash first, or the escapes added below get re-escaped.
escape_sed_replacement() {
    printf '%s' "$1" | sed -e 's/[\\/&]/\\&/g'
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
        hash="$(benchmark_hash_file "$file")"
        [ "$first" -eq 1 ] || printf ','
        printf '{"path":"%s","sha256":"%s","role":"input"}' "$rel" "$hash"
        first=0
    done < <(find "$CAPSULE_ROOT" -type f ! -name worker-manifest.json -print | sort)
    printf '],"source_hash":"%s"}\n' "$(benchmark_hash_file "$CAPSULE_ROOT/planning/SKILL.md")"
} > "$CAPSULE_MANIFEST"
printf '%s\n' '#!/usr/bin/env bash' 'set -euo pipefail' \
    'exec "$@"' > "$CAPSULE_ROOT/plan-context-wrapper.sh"
chmod +x "$CAPSULE_ROOT/plan-context-wrapper.sh"
render_template "$SCRIPT_DIR/worker-prompt.md" "$BENCH_ROOT/worker-prompt.md"
# The worker sees the capsule, never the tagged source checkout. Both operands
# are absolute paths, so this is a literal substring replacement, not a regex.
benchmark_sed_replace "$SRC_ROOT" "$CAPSULE_ROOT" "$BENCH_ROOT/worker-prompt.md"

# benchmark-env.sh is the ONLY channel from this setup half to the case half.
# Each export is printf '%q'-quoted so an arbitrary path survives a re-source.
# Adding, renaming or dropping one means moving start-worker.sh's header list too.
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
# plan-root.sh rule 1: an exported PLANS_ROOT wins over home/project defaults,
# pinning every worker-created plan inside the isolated benchmark workspace.
export PLANS_ROOT=$(printf '%q' "$BENCH_ROOT/.plans")
export REVIEW_MODE=$(printf '%q' "${REVIEW_MODE:-fresh-review}")
export MAX_VERIFICATION_PASSES=$(printf '%q' "${MAX_VERIFICATION_PASSES:-3}")
export MAX_REVIEW_CYCLES=$(printf '%q' "${MAX_REVIEW_CYCLES:-3}")
export BLINDED_ORACLE_SPEC=$(printf '%q' "$BLINDED_ORACLE_SPEC")
# Tailable progress log for the invoking process. PROGRESS_LOG may be
# pre-set by the caller (e.g. run-benchmark.sh); default is a /tmp file
# keyed by RUN_ID so concurrent runs each have their own log.
export PROGRESS_LOG=$(printf '%q' "${PROGRESS_LOG:-${TMPDIR:-/tmp}/ai-skills-benchmark-progress-$RUN_ID.log}")
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

# The case runner is a real, lintable file rather than a 1270-line quoted
# heredoc; it is copied verbatim and reads everything through benchmark-env.sh.
cp "$SCRIPT_DIR/case/start-worker.sh" "$CASE_ROOT/start-worker.sh"

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
