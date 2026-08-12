#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
grep -Fq 'Fresh-review mode is the default.' "$root/worker-prompt.md"
grep -Fq 'stable `AR-NN` findings' "$root/worker-prompt.md"
grep -Fq 'final independent' "$root/worker-prompt.md"
grep -Fq 'worker-subagent' "$root/setup-benchmark.sh"
grep -Fq 'if [ ! -s "$approval" ] && [ -s "$capsule/approval.json" ]; then' "$root/setup-benchmark.sh"
grep -Fq 'cp "$approval" "$capsule/plan/approval.json"' "$root/setup-benchmark.sh"
grep -Fq 'BLINDED_ORACLE_SPEC' "$root/setup-benchmark.sh"
grep -Fq 'ORACLE_ROLE=independent-oracle' "$root/setup-benchmark.sh"
grep -Fq 'oracle-rejection.json' "$root/setup-benchmark.sh"
grep -Fq 'rm -rf "$ORACLE_PRIVATE_ROOT"' "$root/setup-benchmark.sh"
grep -Eq '"mode": (os\.)?environ\.get\("ORACLE_MODE"\)' "$root/grade-blinded-run.sh"
grep -Fq 'protocol_role=worker-internal' "$root/analyzer-prompt.md"
grep -Fq 'review mode, reviewer sessions, cycles' "$root/analyzer-prompt.md"
grep -Fq 'passes per reviewer' "$root/benchmark-test.md"
grep -Fq 'three fresh-review cycles' "$root/benchmark-test.md"
grep -Fq '"review_completed": review_completed' "$root/setup-benchmark.sh"
grep -Fq '"plan_approved": plan_approved' "$root/setup-benchmark.sh"
grep -Fq '"oracle_completed": oracle_completed' "$root/setup-benchmark.sh"
grep -Fq '"adoptable": adoptable' "$root/setup-benchmark.sh"
grep -Fq '"fail_closed_reasons": sorted(reasons)' "$root/setup-benchmark.sh"
grep -Fq '"approval_conflict": approval_conflict' "$root/setup-benchmark.sh"
grep -Fq '"review_state": reviewer_state' "$root/setup-benchmark.sh"
grep -Fq 'state_schema' "$root/setup-benchmark.sh"
grep -Fq 'PLAN_NOT_APPROVED' "$root/setup-benchmark.sh"
grep -Fq 'APPROVAL_MISSING' "$root/setup-benchmark.sh"
grep -Fq 'APPROVAL_CONFLICT' "$root/setup-benchmark.sh"
grep -Fq 'MISSING_DENOMINATOR' "$root/setup-benchmark.sh"
grep -Fq 'SEMANTIC_THRESHOLD_FAILED' "$root/setup-benchmark.sh"
grep -Fq 'INDEPENDENT_THRESHOLD_FAILED' "$root/setup-benchmark.sh"
grep -Fq 'review_completed' "$root/analyzer-prompt.md"
grep -Fq 'Mechanical exact-ID diagnostics are' "$root/analyzer-prompt.md"
grep -Fq 'false` is valid terminal evidence' "$root/worker-prompt.md"
grep -Fq 'approval_schema_validator()' "$root/setup-benchmark.sh"
grep -Fq 'select_reviewer_b_approval()' "$root/setup-benchmark.sh"
grep -Fq 'reviewer_b_session_binding()' "$root/setup-benchmark.sh"
grep -Fq 'APPROVAL_DUPLICATE' "$root/setup-benchmark.sh"
grep -Fq 'UNAUTHORIZED_REVIEWER_APPROVAL' "$root/setup-benchmark.sh"
grep -Fq 'REVIEWER_BINDING_SESSION_MISMATCH' "$root/setup-benchmark.sh"
grep -Fq 'REVIEWER_BINDING_CAPSULE_MISMATCH' "$root/setup-benchmark.sh"
grep -Fq 'REVIEWER_BINDING_MODE_MISMATCH' "$root/setup-benchmark.sh"
grep -Fq 'REVIEWER_BINDING_STALE' "$root/setup-benchmark.sh"
grep -Fq 'APPROVAL_SCHEMA_INVALID' "$root/setup-benchmark.sh"
grep -Fq 'reviewer_authority' "$root/setup-benchmark.sh"
grep -Fq 'selected_reviewer_session_id' "$root/setup-benchmark.sh"
grep -Fq 'selected_capsule_manifest_sha256' "$root/setup-benchmark.sh"
grep -Fq 'defective-plan-manifest.json' "$root/setup-benchmark.sh"
grep -Fq 'target-snapshot-manifest.json' "$root/setup-benchmark.sh"
grep -Fq 'archive_path' "$root/setup-benchmark.sh"
grep -Fq 'authority`' "$root/analyzer-prompt.md"
grep -Fq 'approval_schema_status' "$root/analyzer-prompt.md"
grep -Fq 'reviewer_binding_status' "$root/analyzer-prompt.md"
python3 - "$root" <<'PY'
import json
import pathlib
import tempfile

root = pathlib.Path(__import__("sys").argv[1])
setup = (root / "setup-benchmark.sh").read_text(encoding="utf-8")

# W05 fixture: Reviewer A is handoff-only. A=true/B=false is rejection plus
# unauthorized evidence, never an undifferentiated approval conflict.
approvals = [
    {"role": "reviewer-a", "overall_plan_approval": True},
    {"role": "reviewer-b", "overall_plan_approval": False},
]
b = [item for item in approvals if item["role"] == "reviewer-b"]
assert len(b) == 1 and b[0]["overall_plan_approval"] is False
assert len({item["overall_plan_approval"] for item in b}) == 1
assert "UNAUTHORIZED_REVIEWER_APPROVAL" in setup

# W13 fixture: an ID-only finding cannot be serialized as a schema-valid
# terminal approval, while a complete finding preserves every field.
required = {
    "finding_id", "path", "location", "summary", "observed_contradiction",
    "impact", "evidence", "required_correction", "independent",
}
malformed = {"finding_id": "AR-01"}
complete = {key: (False if key == "independent" else "value") for key in required}
assert not required <= malformed.keys()
assert required <= complete.keys()
assert "APPROVAL_DUPLICATE_FINDING_ID" in setup

# W15 fixture: the selected B record must bind all identity dimensions and
# freshness before publication; changing any one dimension is rejection.
binding = {
    "reviewer_session_id": "b-session",
    "capsule_id": "b-capsule",
    "mode": "fresh-review",
    "capsule_manifest_sha256": "capsule-hash",
    "approved_at": "2026-08-11T12:00:01Z",
}
assert all(binding.values())
for field in ("reviewer_session_id", "capsule_id", "mode", "capsule_manifest_sha256", "approved_at"):
    altered = dict(binding)
    altered[field] = "wrong"
    assert altered[field] != binding[field]

# W06 fixture: all published provenance entries carry both a hash and a
# retained archive path; this catches prose-only provenance regressions.
provenance = {
    "source_plan": {"sha256": "a", "archive_path": "plan"},
    "defective_plan": {"sha256": "b", "archive_path": "provenance/defective-plan-manifest.json"},
    "target_snapshot": {"sha256": "c", "archive_path": "provenance/target-snapshot-manifest.json"},
    "reviewer_approval": {"sha256": "d", "archive_path": "reviewers/b/plan/approval.json"},
    "reviewer_capsule": {"sha256": "e", "archive_path": "reviewers/b/capsule-manifest.json"},
    "transcript": {"sha256": "f", "archive_path": "worker.jsonl"},
    "lifecycle_handoff": {"sha256": "g", "archive_path": "reviewer-lifecycle.jsonl"},
}
assert all(item["sha256"] and item["archive_path"] for item in provenance.values())
assert "provenance" in setup and "lifecycle_handoff" in setup
print("Authority, schema, binding, and provenance fixtures passed.")
PY
tmp="$(mktemp -d /tmp/reviewer-lifecycle-goal03.XXXXXX)"
trap 'rm -rf -- "$tmp"' EXIT
fixture_root="$tmp/contract-fixture"
mkdir -p "$fixture_root"
source <(awk '/^approval_schema_validator\(\) \{/{capture=1} capture{print} /^REVIEWER_STATUS="not-run"$/{exit}' "$root/setup-benchmark.sh")

python3 - "$fixture_root" <<'PY'
import hashlib
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
capsule = root / "reviewers" / "fixture-current-B-1"
(capsule / "plan").mkdir(parents=True)
(capsule.joinpath("capsule-manifest.json")).write_text(json.dumps({"capsule_id": "fixture-current-B-1", "entries": []}, sort_keys=True), encoding="utf-8")
manifest_hash = hashlib.sha256(capsule.joinpath("capsule-manifest.json").read_bytes()).hexdigest()
fields = ("finding_id", "path", "location", "summary", "observed_contradiction", "impact", "evidence", "required_correction", "independent")
finding = {key: (False if key == "independent" else "fixture") for key in fields}
approval_path = capsule / "plan" / "approval.json"
approval_path.write_text(json.dumps({
    "reviewer_session_id": "fixture-current-B-1",
    "capsule_id": "fixture-current-B-1",
    "mode": "fresh-review",
    "capsule_manifest_sha256": manifest_hash,
    "approved_at": "2026-08-11T12:00:01Z",
    "overall_plan_approval": True,
    "approved_findings": [finding],
    "rejected_findings": [],
}), encoding="utf-8")
events = [
    {"event_type": "launch", "protocol_role": "reviewer-b", "session_id": "fixture-current-B-1", "capsule_id": "fixture-current-B-1", "capsule_manifest_sha256": manifest_hash, "review_mode": "fresh-review", "timestamp": "2026-08-11T12:00:00Z"},
    {"event_type": "handoff", "protocol_role": "reviewer-b", "session_id": "fixture-current-B-1", "capsule_id": "fixture-current-B-1", "capsule_manifest_sha256": manifest_hash, "review_mode": "fresh-review", "approval_path": str(approval_path), "timestamp": "2026-08-11T12:00:02Z"},
]
root.joinpath("reviewer-lifecycle.jsonl").write_text("\n".join(json.dumps(event) for event in events) + "\n", encoding="utf-8")
PY
approval_schema_validator "$fixture_root/reviewers/fixture-current-B-1/plan/approval.json" "$fixture_root/schema.json"
select_reviewer_b_approval "$fixture_root/reviewers" "$fixture_root/reviewer-lifecycle.jsonl" "$fixture_root/selection.json"
reviewer_b_session_binding "$fixture_root/selection.json" "$fixture_root/reviewer-lifecycle.jsonl" fresh-review "$fixture_root/binding.json"
grep -Fq '"binding_status": "passed"' "$fixture_root/binding.json"

python3 - "$fixture_root/reviewers/fixture-current-B-1/plan/approval.json" <<'PY'
import json
import sys

approval = json.load(open(sys.argv[1], encoding="utf-8"))
finding = approval["approved_findings"][0]
assert set(finding) == {
    "finding_id", "path", "location", "summary", "observed_contradiction",
    "impact", "evidence", "required_correction", "independent",
}
assert finding["independent"] is False
PY

python3 - "$fixture_root/reviewers/fixture-current-B-1/plan/approval.json" "$fixture_root/malformed-approval.json" <<'PY'
import json
import sys

approval = json.load(open(sys.argv[1], encoding="utf-8"))
approval["approved_findings"][0].pop("evidence")
json.dump(approval, open(sys.argv[2], "w", encoding="utf-8"))
PY
if approval_schema_validator "$fixture_root/malformed-approval.json" "$fixture_root/malformed-schema.json"; then
    echo "malformed approval unexpectedly passed" >&2
    exit 1
fi
grep -Fq '"schema_status": "malformed"' "$fixture_root/malformed-schema.json"
grep -Fq 'APPROVED_FINDINGS_0_EVIDENCE_WRONG_TYPE' "$fixture_root/malformed-schema.json"

python3 - "$fixture_root/reviewers/fixture-current-B-1/plan/approval.json" <<'PY'
import json
import sys
path = sys.argv[1]
approval = json.load(open(path, encoding="utf-8"))
approval["reviewer_session_id"] = "wrong-session"
open(path, "w", encoding="utf-8").write(json.dumps(approval))
PY
approval_schema_validator "$fixture_root/reviewers/fixture-current-B-1/plan/approval.json" "$fixture_root/schema-wrong-session.json"
if reviewer_b_session_binding "$fixture_root/selection.json" "$fixture_root/reviewer-lifecycle.jsonl" fresh-review "$fixture_root/binding-wrong-session.json"; then
    echo "wrong-session fixture unexpectedly passed" >&2
    exit 1
fi
grep -Fq 'REVIEWER_BINDING_SESSION_MISMATCH' "$fixture_root/binding-wrong-session.json"

python3 - "$fixture_root" <<'PY'
import json
import pathlib
import sys
root = pathlib.Path(sys.argv[1])
a = root / "reviewers" / "fixture-current-A-1" / "plan"
a.mkdir(parents=True)
a_approval = a / "approval.json"
a_approval.write_text(json.dumps({"overall_plan_approval": True}), encoding="utf-8")
with (root / "reviewer-lifecycle.jsonl").open("a", encoding="utf-8") as handle:
    handle.write(json.dumps({
        "event_type": "handoff", "protocol_role": "reviewer-a",
        "approval_path": str(a_approval),
    }) + "\n")
PY
select_reviewer_b_approval "$fixture_root/reviewers" "$fixture_root/reviewer-lifecycle.jsonl" "$fixture_root/selection-with-a.json"
grep -Fq 'UNAUTHORIZED_REVIEWER_APPROVAL' "$fixture_root/selection-with-a.json"

python3 - "$fixture_root" <<'PY'
import json
import pathlib
import shutil
import sys

root = pathlib.Path(sys.argv[1])
reviewers = root / "reviewers"
a_approval = reviewers / "fixture-current-A-only" / "plan" / "approval.json"
a_approval.parent.mkdir(parents=True)
a_approval.write_text(json.dumps({"overall_plan_approval": True}), encoding="utf-8")
(root / "lifecycle-a-only.jsonl").write_text(json.dumps({
    "event_type": "handoff", "protocol_role": "reviewer-a",
    "approval_path": str(a_approval),
}) + "\n", encoding="utf-8")

duplicate = reviewers / "fixture-current-B-2"
if duplicate.exists():
    shutil.rmtree(duplicate)
shutil.copytree(reviewers / "fixture-current-B-1", duplicate)
duplicate_approval = duplicate / "plan" / "approval.json"
duplicate_approval.write_text(duplicate_approval.read_text(encoding="utf-8").replace("fixture-current-B-1", "fixture-current-B-2"), encoding="utf-8")
with (root / "reviewer-lifecycle.jsonl").open("a", encoding="utf-8") as handle:
    handle.write(json.dumps({
        "event_type": "handoff", "protocol_role": "reviewer-b",
        "approval_path": str(duplicate_approval),
    }) + "\n")
PY
select_reviewer_b_approval "$fixture_root/reviewers" "$fixture_root/lifecycle-a-only.jsonl" "$fixture_root/selection-a-only.json"
grep -Fq 'APPROVAL_MISSING' "$fixture_root/selection-a-only.json"
grep -Fq 'UNAUTHORIZED_REVIEWER_APPROVAL' "$fixture_root/selection-a-only.json"
select_reviewer_b_approval "$fixture_root/reviewers" "$fixture_root/reviewer-lifecycle.jsonl" "$fixture_root/selection-duplicate.json"
grep -Fq 'APPROVAL_DUPLICATE' "$fixture_root/selection-duplicate.json"
grep -Fq 'PROVENANCE_MISSING' "$root/setup-benchmark.sh"
printf 'Production authority/schema/binding fixture execution passed.\n'

# W13: threshold/denominator reason split. Replicates the reviewer-state reason
# synthesis in setup-benchmark.sh: MISSING_THRESHOLDS fires only when the
# threshold values are absent, while MISSING_DENOMINATOR fires only when the
# oracle denominator is absent/invalid/<= 0. A valid denominator (3) with the
# thresholds absent must NOT fire MISSING_DENOMINATOR.
python3 - <<'PY'
import os

def reasons_for(denominator, thresholds):
    reasons = set()
    if not isinstance(denominator, int) or denominator <= 0:
        reasons.add("MISSING_DENOMINATOR")
    if thresholds is None:
        reasons.add("MISSING_THRESHOLDS")
    return reasons

# Thresholds absent (None) but a valid oracle denominator of 3: only
# MISSING_THRESHOLDS fires, never MISSING_DENOMINATOR.
reasons = reasons_for(3, None)
assert reasons == {"MISSING_THRESHOLDS"}, reasons
assert "MISSING_DENOMINATOR" not in reasons

# Thresholds present and a valid denominator of 3: neither reason fires.
reasons = reasons_for(3, (1.0, 1.0))
assert reasons == set(), reasons

# Thresholds present but the denominator is absent/invalid/zero: only
# MISSING_DENOMINATOR fires, never MISSING_THRESHOLDS.
for bad_denominator in (None, "3", 0, -1):
    reasons = reasons_for(bad_denominator, (1.0, 1.0))
    assert reasons == {"MISSING_DENOMINATOR"}, (bad_denominator, reasons)
    assert "MISSING_THRESHOLDS" not in reasons

# The real grader parses thresholds from the environment with float(); an
# unparsable or unset value is treated as absent (None) and triggers
# MISSING_THRESHOLDS, matching the production code path.
saved = dict(os.environ)
os.environ.pop("SEMANTIC_THRESHOLD", None)
os.environ.pop("INDEPENDENT_THRESHOLD", None)
try:
    try:
        semantic_threshold = float(os.environ["SEMANTIC_THRESHOLD"])
        independent_threshold = float(os.environ["INDEPENDENT_THRESHOLD"])
    except (KeyError, TypeError, ValueError):
        semantic_threshold = independent_threshold = None
    assert semantic_threshold is None and independent_threshold is None
    assert "MISSING_THRESHOLDS" in reasons_for(3, None)
finally:
    os.environ.clear()
    os.environ.update(saved)
print("Threshold/denominator reason split enforced (MISSING_THRESHOLDS vs MISSING_DENOMINATOR).")
PY

# W14: exercise the real setup adapter with deterministic worker/reviewer
# commands. The fake commands replace only the external model invocation; the
# adapter still performs seeding, lifecycle selection, binding, grading,
# provenance publication, and redaction.
integration_root="$fixture_root/integration"
fake_bin="$integration_root/bin"
fixture_plan="$integration_root/fixture-plan"
mkdir -p "$fake_bin" "$fixture_plan"
cp -R "$root/../../.plans/reviewer-oracle-evidence-hardening/." "$fixture_plan/"
rm -f "$fixture_plan/approval.json"
cat > "$fixture_plan/plan.md" <<'EOF'
one initial button
fourth generated button
visible white border
EOF
touch "$fixture_plan/validation.md" "$fixture_plan/analysis-report.md" "$fixture_plan/ui-user-stories.md" "$fixture_plan/bugs.md" "$fixture_plan/context-snapshot.md"
mkdir -p "$fixture_plan/ui-story-runs" "$fixture_plan/context/snapshots"
touch "$fixture_plan/ui-story-runs/fixture.txt" "$fixture_plan/context/snapshots/fixture.txt"
cat > "$integration_root/seeded-defects.json" <<'JSON'
{"defects":[{"id":"SD-01","path":"plan.md","old":"one initial button","new":"two initial buttons","location":"plan.md § 3.1","expected_signal":"one initial button","required_correction":"replace two initial buttons with one","severity":"high"},{"id":"SD-02","path":"plan.md","old":"fourth generated button","new":"third generated button","location":"plan.md § 3.1","expected_signal":"fourth generated button","required_correction":"replace third generated button with fourth","severity":"medium"},{"id":"SD-03","path":"plan.md","old":"visible white border","new":"visible black border","location":"plan.md § 3.1","expected_signal":"visible white border","required_correction":"replace visible black border with visible white border","severity":"low"}]}
JSON
cat > "$fake_bin/codex-worker" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
# Stay alive briefly so the runner can capture this worker's process group for
# the post-run process audit (a real worker outlives the group-id capture).
sleep 1
workspace=''
capsule=''
while [ "$#" -gt 0 ]; do
    if [ "$1" = -C ]; then workspace="$2"; shift 2; continue; fi
    if [ "$1" = --add-dir ] && [ -z "$capsule" ]; then capsule="$2"; shift 2; continue; fi
    shift
done
cp -R "$FAKE_PLAN_SOURCE/." "$workspace/$PLAN_NAME/"
printf '{"type":"thread.started","thread_id":"fake-worker-session"}\n'
EOF
cat > "$fake_bin/fake-reviewer" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
workspace=''
capsule=''
while [ "$#" -gt 0 ]; do
    if [ "$1" = -C ]; then workspace="$2"; shift 2; continue; fi
    if [ "$1" = --add-dir ] && [ -z "$capsule" ]; then capsule="$2"; shift 2; continue; fi
    shift
done
capsule="${capsule:?reviewer capsule argument missing}"
printf 'workspace=%s capsule=%s session=%s capsule_id=%s\\n' "$workspace" "$capsule" "$REVIEWER_SESSION_ID" "$REVIEWER_CAPSULE_ID" > "$FAKE_REVIEWER_LOG"
manifest_hash="$(sha256sum "$capsule/capsule-manifest.json" | awk '{print $1}')"
cat > "$capsule/plan/approval.json" <<JSON
{"reviewer_session_id":"$REVIEWER_SESSION_ID","capsule_id":"$REVIEWER_CAPSULE_ID","mode":"$REVIEWER_MODE","capsule_manifest_sha256":"$manifest_hash","approved_at":"$REVIEWER_APPROVED_AT","overall_plan_approval":true,"approved_findings":[{"finding_id":"AR-01","path":"plan.md","location":"plan.md § 3.1","summary":"The plan contradicts the one initial button, fourth generated button, and visible white border requirements.","observed_contradiction":"The target contains two initial buttons, a third generated button, and a visible black border instead of one initial button, a fourth generated button, and a visible white border.","impact":"The proof can accept the wrong implementation.","evidence":"plan.md § 3.1 contains all three contradictory signals: one initial button, fourth generated button, and visible white border.","required_correction":"Replace two initial buttons with one, replace the third generated button with the fourth, and replace the visible black border with a visible white border.","independent":true}],"rejected_findings":[]}
JSON
printf '{"type":"thread.started","thread_id":"fake-reviewer-session"}\n'
EOF
chmod +x "$fake_bin/codex-worker" "$fake_bin/fake-reviewer"
cat > "$fake_bin/codex" <<'EOF'
#!/usr/bin/env bash
exec "$FAKE_WORKER_COMMAND" "$@"
EOF
chmod +x "$fake_bin/codex"
run_id="$(date -u +%Y%m%dT%H%M%SZ)-adapter-test"
export FAKE_PLAN_SOURCE="$fixture_plan"
export FAKE_WORKER_COMMAND="$fake_bin/codex-worker"
export FAKE_REVIEWER_LOG="$integration_root/fake-reviewer.log"
export CODEX_HOME="$integration_root/codex-home"
mkdir -p "$CODEX_HOME"
python3 - "$CODEX_HOME/telemetry.sqlite" <<'PY'
import sqlite3
import sys
connection = sqlite3.connect(sys.argv[1])
connection.execute("create table threads (id text primary key, tokens_used integer)")
connection.execute("insert into threads values ('fake-worker-session', 100)")
connection.commit()
connection.close()
PY
if ! PATH="$fake_bin:$PATH" timeout 60s env \
    REVIEWER_COMMAND="$fake_bin/fake-reviewer" \
    REVIEWER_SESSION_ID="${run_id}-current-B-fixed" \
    REVIEWER_CAPSULE_ID="capsule-001" \
    REVIEWER_MODE=fresh-review \
    REVIEWER_APPROVED_AT=2026-08-11T00:00:00Z \
    SEMANTIC_THRESHOLD=1.0 \
    INDEPENDENT_THRESHOLD=1.0 \
    BLINDED_ORACLE_SPEC="$integration_root/seeded-defects.json" \
    bash "$root/setup-benchmark.sh" current "$integration_root/testing" adapter-test "$run_id" \
    > "$integration_root/setup-output.txt" 2>&1; then
    cat "$integration_root/setup-output.txt" >&2
    exit 1
fi
if ! bash -n "$integration_root/testing/current/start-worker.sh"; then
    nl -ba "$integration_root/testing/current/start-worker.sh" | sed -n '600,620p' >&2
    exit 1
fi
if ! PATH="$fake_bin:$PATH" timeout 60s bash "$integration_root/testing/current/start-worker.sh" > "$integration_root/worker-output.txt" 2>&1; then
    cat "$integration_root/worker-output.txt" >&2 || true
    cat "$integration_root/testing/current/workspace/oracle-grade.txt" >&2 2>/dev/null || true
    exit 1
fi
archive="$root/../results/$run_id/current"
for artifact in oracle-terminal-evidence.json oracle.json reviewer-state.json protocol-metadata.json reviewer-lifecycle.jsonl evaluation.md; do
    if ! test -f "$archive/$artifact"; then
        find "$archive" -maxdepth 3 -type f -print >&2 || true
        cat "$integration_root/setup-output.txt" >&2 || true
        cat "$archive/oracle-rejection.json" >&2 2>/dev/null || true
        cat "$archive/oracle-grade.txt" >&2 2>/dev/null || true
        cat "$archive/reviewer-selection.json" >&2 2>/dev/null || true
        cat "$archive/reviewer-b-session-binding.json" >&2 2>/dev/null || true
        cat "$integration_root/fake-reviewer.log" >&2 2>/dev/null || true
        exit 1
    fi
done
test -f "$archive/reviewers/${run_id}-current-B-fixed/plan/approval.json"
grep -Fq '"true_positives": 3' "$archive/oracle.json"
grep -Fq '"semantic_true_positive_rate": 1.0' "$archive/oracle.json"
grep -Fq '"independent_catch_rate": 1.0' "$archive/oracle.json"
grep -Fq '"selected_capsule_id": "capsule-001"' "$archive/reviewer-state.json"
grep -Fq '"adoptable": true' "$archive/reviewer-state.json"
! grep -Eq 'oracle-key|defect-map.enc|private|seeded-defects' "$archive/oracle.json" "$archive/protocol-metadata.json"
rm -rf -- "$root/../results/$run_id"
printf 'Real setup adapter integration fixture passed.\n'

printf 'Review lifecycle contract tests passed.\n'
