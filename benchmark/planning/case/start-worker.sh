#!/usr/bin/env bash
# start-worker.sh - run one prepared planning benchmark case end to end.
#
# Copied verbatim into <case-root>/start-worker.sh by setup-benchmark.sh, which
# is the only writer of this file's single input channel: the sibling
# benchmark-env.sh. Every value below is emitted there printf '%q'-quoted, so
# this script may assume each name is set (possibly to the empty string) and
# must never re-derive one from the host environment.
#
# Usage:
#   <case-root>/start-worker.sh
#
# Exit code is the WORKER's exit code, not the run verdict: a fully tainted run
# with a clean worker still exits 0. The verdict lives in telemetry.json's
# "status" and reviewer-state.json's "adoptable".
#
# benchmark-env.sh interface - 30 exports, all of which must keep existing.
# 25 are read by name in this file, in emission order:
#   REPO_ROOT                 TAG                       REVISION
#   RUN_ID                    CASE_ROOT                 SRC_ROOT
#   BENCH_ROOT                PLAN_NAME                 RESULT_DIR
#   STAGING_RESULT_DIR        PROTOCOL_ID               COHORT
#   WORKER_CAPSULE            CAPSULE_ROOT              CAPSULE_BASE
#   REVIEW_MODE               BLINDED_ORACLE_SPEC       PROGRESS_LOG
#   REVIEWER_COMMAND          REVIEWER_SESSION_ID       REVIEWER_CAPSULE_ID
#   REVIEWER_MODE             REVIEWER_APPROVED_AT      SEMANTIC_THRESHOLD
#   INDEPENDENT_THRESHOLD
#
# 5 are never named here and must not be removed from the emitter either:
#   BENCHMARK_AGENT           read by runtime/lib-agent.sh's resolver, sourced below
#   PLANS_ROOT                read by the worker's own plan-root.sh, inherited
#   WORKER_WORKSPACE          unread by any case script (equals BENCH_ROOT)
#   MAX_VERIFICATION_PASSES   unread by any case script
#   MAX_REVIEW_CYCLES         unread by any case script
#
# Optional host environment, deliberately NOT part of that interface:
#   TMPDIR, WORKER_TIMEOUT, REVIEWER_TIMEOUT

set -euo pipefail

source "$(dirname "$0")/benchmark-env.sh"

# Resolve the active agent driver through the shared runtime. REPO_ROOT is
# exported by benchmark-env.sh; lib-agent.sh sources the resolver and driver
# and exports the launcher (setsid/timeout/mode) and argv builders.
source "$REPO_ROOT/benchmark/planning/runtime/lib-agent.sh"
# shellcheck source=../lib-portable.sh
source "$REPO_ROOT/benchmark/planning/lib-portable.sh"
# python3 drives the approval checks, state synthesis, telemetry and metadata
# below. Refuse once, here, rather than eleven interpreter tracebacks deep.
benchmark_require_python3 || exit 69
# shellcheck source=../lib-process.sh
source "$REPO_ROOT/benchmark/planning/lib-process.sh"
# shellcheck source=../lib-structural-gate.sh
source "$REPO_ROOT/benchmark/planning/lib-structural-gate.sh"

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

# Tailable progress log for the invoking process: one timestamped line per
# stage transition (preflight, worker, validation, review, oracle, publish).
progress_log() {
    printf '[%s] %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" >> "$PROGRESS_LOG"
}

# Scratch roots honour TMPDIR the way benchmark-env.sh's CAPSULE_BASE does; both
# are removed before publication so a run leaves nothing behind in the temp dir.
SCRATCH_BASE="${TMPDIR:-/tmp}"
REVIEWER_WORKSPACE_ROOT="$SCRATCH_BASE/ai-skills-reviewer-workspaces/$RUN_ID/$REVISION"

START_EPOCH="$(date -u +%s)"
START="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

PROCESS_AUDIT_STATE="$CASE_ROOT/process-audit-state.txt"
process_audit_probe "$PROCESS_AUDIT_STATE"

benchmark_basenames "$BENCH_ROOT" | LC_ALL=C sort > "$CASE_ROOT/preflight-before-worker.txt"
progress_log "case $REVISION ($TAG): preflight ready; starting worker"

# The handler reads these two names live (see lib-process.sh); they must exist
# before the trap is armed and the pid must be cleared once the worker is reaped.
PROCESS_CLEANUP_CHILD_PID=""
PROCESS_CLEANUP_GROUP_ID=""
trap process_cleanup_on_signal INT TERM

persona_bootstrap worker || exit 64
persona_bootstrap_prompt "$BENCH_ROOT/worker-prompt.md" worker "$REPO_ROOT/planning/roles/VOICES.md" || exit 64
agent_argv_worker "$BENCH_ROOT" "$WORKER_CAPSULE" "$BENCH_ROOT/worker-prompt.md"
launch_agent setsid "${WORKER_TIMEOUT:-45m}" "$BENCH_ROOT/worker.jsonl"
PROCESS_CLEANUP_CHILD_PID="$AGENT_PID"
PROCESS_CLEANUP_GROUP_ID="$AGENT_PGID"
WORKER_PROCESS_GROUP_ID="$AGENT_PGID"
wait_agent
CODE="$AGENT_EXIT"
progress_log "case $REVISION ($TAG): worker exited code=$CODE"
PROCESS_CLEANUP_CHILD_PID=""

END_EPOCH="$(date -u +%s)"
END="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
ELAPSED="$((END_EPOCH - START_EPOCH))"

PROCESS_AUDIT="$(process_audit "$PROCESS_AUDIT_STATE" "$WORKER_PROCESS_GROUP_ID" "$CASE_ROOT")"
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
    ORACLE_PRIVATE_ROOT="$SCRATCH_BASE/ai-skills-oracle-private/$RUN_ID/$REVISION"
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
# The gate's verdict arrives as a return code, never as a variable shared across
# the redirection - see lib-structural-gate.sh's docblock for why that matters.
structural_gate_report "$PLAN_DIR" "$PLAN_NAME" "$PLAN_FOUND" \
    > "$STRUCTURAL_REPORT" || STRUCTURAL_VALIDATION="fail"
progress_log "case $REVISION ($TAG): structural validation=$STRUCTURAL_VALIDATION"

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
# Thin wrappers over lib-approval.py, kept so the exit-code contract stays
# visible at the call sites: 0 = check passed, 65 = malformed input or a
# rejected identity binding.
LIB_APPROVAL="$REPO_ROOT/benchmark/planning/lib-approval.py"
approval_schema_validator() {
    python3 "$LIB_APPROVAL" schema-validate "$1" "$2"
}
select_reviewer_b_approval() {
    python3 "$LIB_APPROVAL" select-approval "$1" "$2" "$3"
}
reviewer_b_session_binding() {
    python3 "$LIB_APPROVAL" bind-session "$1" "$2" "$3" "$4"
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
        session="${RUN_ID}-${REVISION}-${role}-$(benchmark_unique_suffix)"
        capsule_id="$session"
        review_mode="${REVIEW_MODE:-fresh-review}"
        approved_at="${REVIEWER_APPROVED_AT:-}"
    fi
    capsule="$CAPSULE_BASE/$RUN_ID/$REVISION/reviewers/$session"
    workspace="$REVIEWER_WORKSPACE_ROOT/$session/workspace"
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
    capsule_manifest_sha256="$(benchmark_hash_file "$capsule/capsule-manifest.json")"
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
        # Reviewers write the handoff either beside their capsule plan or in
        # the plan directory; accept both, then archive the canonical copy.
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
REVIEW_FINDING_COUNTS="$(python3 - "$REVIEWER_B_APPROVAL" <<'PY'
import json
import sys
try:
    value = json.load(open(sys.argv[1], encoding="utf-8"))
except (OSError, json.JSONDecodeError):
    value = {}
approved = value.get("approved_findings")
rejected = value.get("rejected_findings")
approved_count = len(approved) if isinstance(approved, list) else 0
rejected_count = len(rejected) if isinstance(rejected, list) else 0
print(f"approved={approved_count} rejected={rejected_count} total={approved_count + rejected_count}")
PY
)"
progress_log "case $REVISION ($TAG): review complete (mode=${REVIEW_MODE:-fresh-review}); findings $REVIEW_FINDING_COUNTS"

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
progress_log "case $REVISION ($TAG): oracle status=$ORACLE_STATUS"
PROVENANCE_MATERIAL="$BENCH_ROOT/provenance-material.json"
DEFECTIVE_PLAN_PATH=""
TARGET_SNAPSHOT_PATH=""
if [ -n "$ORACLE_TARGET_ROOT" ] && [ -n "$ORACLE_PRIVATE_ROOT" ]; then
    DEFECTIVE_PLAN_PATH="$ORACLE_PLAN_DIR"
    TARGET_SNAPSHOT_PATH="$ORACLE_PRIVATE_ROOT/target-snapshot"
fi
python3 "$REPO_ROOT/benchmark/planning/synthesize-state.py" provenance \
    "$PROVENANCE_MATERIAL" "$PLAN_DIR" "$DEFECTIVE_PLAN_PATH" "$TARGET_SNAPSHOT_PATH" \
    "$REVIEWER_B_APPROVAL" "${REVIEWER_B_TRANSCRIPT:-$BENCH_ROOT/worker.jsonl}" \
    "$REVIEWER_LIFECYCLE" "$REVIEWER_ROOT" "$REVIEWER_BINDING" "$REVIEWER_SELECTION"
# Both scratch trees have served their purpose once provenance has digested the
# reviewer transcript; nothing below reads either again. The reviewer workspaces
# used to leak one tree per run because only the oracle root was ever removed.
if [ -n "$ORACLE_PRIVATE_ROOT" ]; then
    rm -rf "$ORACLE_PRIVATE_ROOT"
fi
rm -rf "$REVIEWER_WORKSPACE_ROOT"
# Each scratch root is <base>/<run-id>/<revision>; drop the now-empty run-id
# level too. rmdir refuses a non-empty directory, which is exactly the guard
# needed when a sibling revision of the same run is still going.
rmdir "$SCRATCH_BASE/ai-skills-reviewer-workspaces/$RUN_ID" \
      "$SCRATCH_BASE/ai-skills-oracle-private/$RUN_ID" 2>/dev/null || true

# Approval is terminal review evidence, not an adoption decision. Keep this
# state calculation independent from worker/oracle exit status so a valid
# negative approval remains gradeable while adoption fails closed.
REVIEWER_STATE="$BENCH_ROOT/reviewer-state.json"
export REVIEWER_STATUS ORACLE_STATUS STATUS REVIEWER_STATE
SYNTHESIZE_STATE="$REPO_ROOT/benchmark/planning/synthesize-state.py"
python3 "$SYNTHESIZE_STATE" state "$REVIEWER_STATE" "$BENCH_ROOT/oracle.json" \
    "$REVIEWER_SELECTION" "$REVIEWER_SCHEMA" "$REVIEWER_BINDING" "$PROVENANCE_MATERIAL"

# Preserve one machine-readable, provenance-bearing telemetry artifact next to
# the human-readable lookup. Values unavailable from the runner remain null or
# explicitly unavailable; this file never substitutes inferred totals.
export PROTOCOL_ID RUN_ID REVISION SESSION_ID STATUS REVIEW_MODE START END ELAPSED
export CODE VALIDATION STRUCTURAL_VALIDATION PROCESS_AUDIT HTML_COUNT REVIEWER_STATUS
export TELEMETRY_SOURCE TELEMETRY_STATUS TOTAL_USAGE USAGE_RECORDS REVIEWER_LIFECYCLE ORACLE_STATUS REVIEWER_STATE
python3 "$REPO_ROOT/benchmark/planning/emit-telemetry.py" > "$BENCH_ROOT/telemetry.json"
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
progress_log "case $REVISION ($TAG): published result to $RESULT_DIR"

printf 'completed %s code=%s status=%s result=%s\n' "$REVISION" "$CODE" "$STATUS" "$RESULT_DIR"
exit "$CODE"
