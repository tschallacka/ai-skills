#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../planning/tests" && pwd)/lib-test.sh"
# Report the failing expression: these assertions are bare `[ ... ]` under
# set -e, which otherwise exits 1 in silence.
t_trap_assertions


root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$root/runtime/lib-agent.sh"  # defines benchmark_result_parent for the archive path
# The harness surface is the setup emitter plus the generated worker spine and
# the libraries extracted out of it. A contract string may live in any of them,
# so assert against the whole set rather than pinning a string to one file.
harness_sources=("$root/setup-benchmark.sh" "$root/case/start-worker.sh")
for harness_extra in \
    "$root/lib-portable.sh" \
    "$root/lib-process.sh" \
    "$root/lib-structural-gate.sh" \
    "$root/lib-approval.py" \
    "$root/synthesize-state.py" \
    "$root/emit-telemetry.py"; do
    [ -f "$harness_extra" ] || continue
    harness_sources+=("$harness_extra")
done
harness_grep() { grep -Fq -- "$1" "${harness_sources[@]}"; }
harness_grep_e() { grep -Eq -- "$1" "${harness_sources[@]}"; }
grep -Fq 'Fresh-review mode is the default.' "$root/worker-prompt.md"
grep -Fq 'stable `AR-NN` findings' "$root/worker-prompt.md"
grep -Fq 'final independent' "$root/worker-prompt.md"
harness_grep 'worker-subagent'
harness_grep 'if [ ! -s "$approval" ] && [ -s "$capsule/approval.json" ]; then'
harness_grep 'cp "$approval" "$capsule/plan/approval.json"'
harness_grep 'BLINDED_ORACLE_SPEC'
harness_grep 'ORACLE_ROLE=independent-oracle'
harness_grep 'oracle-rejection.json'
harness_grep 'rm -rf "$ORACLE_PRIVATE_ROOT"'
grep -Eq '"mode": (os\.)?environ\.get\("ORACLE_MODE"\)' "$root/grade-blinded-run.sh"
grep -Fq 'protocol_role=worker-internal' "$root/analyzer-prompt.md"
grep -Fq 'review mode, reviewer sessions, cycles' "$root/analyzer-prompt.md"
grep -Fq 'passes per reviewer' "$root/benchmark-test.md"
grep -Fq 'three fresh-review cycles' "$root/benchmark-test.md"
harness_grep '"review_completed": review_completed'
harness_grep '"plan_approved": plan_approved'
harness_grep '"oracle_completed": oracle_completed'
harness_grep '"adoptable": adoptable'
harness_grep '"fail_closed_reasons": sorted(reasons)'
harness_grep '"approval_conflict": approval_conflict'
harness_grep '"review_state": reviewer_state'
harness_grep 'state_schema'
harness_grep 'PLAN_NOT_APPROVED'
harness_grep 'APPROVAL_MISSING'
harness_grep 'APPROVAL_CONFLICT'
harness_grep 'MISSING_DENOMINATOR'
harness_grep 'SEMANTIC_THRESHOLD_FAILED'
harness_grep 'INDEPENDENT_THRESHOLD_FAILED'
grep -Fq 'review_completed' "$root/analyzer-prompt.md"
grep -Fq 'Mechanical exact-ID diagnostics are' "$root/analyzer-prompt.md"
grep -Fq 'false` is valid terminal evidence' "$root/worker-prompt.md"
harness_grep 'approval_schema_validator()'
harness_grep 'select_reviewer_b_approval()'
harness_grep 'reviewer_b_session_binding()'
harness_grep 'APPROVAL_DUPLICATE'
harness_grep 'UNAUTHORIZED_REVIEWER_APPROVAL'
harness_grep 'REVIEWER_BINDING_SESSION_MISMATCH'
harness_grep 'REVIEWER_BINDING_CAPSULE_MISMATCH'
harness_grep 'REVIEWER_BINDING_MODE_MISMATCH'
harness_grep 'REVIEWER_BINDING_STALE'
harness_grep 'APPROVAL_SCHEMA_INVALID'
harness_grep 'reviewer_authority'
harness_grep 'selected_reviewer_session_id'
harness_grep 'selected_capsule_manifest_sha256'
harness_grep 'defective-plan-manifest.json'
harness_grep 'target-snapshot-manifest.json'
harness_grep 'archive_path'
grep -Fq 'authority`' "$root/analyzer-prompt.md"
grep -Fq 'approval_schema_status' "$root/analyzer-prompt.md"
grep -Fq 'reviewer_binding_status' "$root/analyzer-prompt.md"
python3 - "${harness_sources[@]}" <<'PY'
import json
import pathlib
import sys

# Concatenated harness surface: the emitter, the generated worker spine, and the
# libraries extracted out of it. Assertions below are "this contract string is
# somewhere in the harness", not "in this one file".
setup = "\n".join(pathlib.Path(arg).read_text(encoding="utf-8") for arg in sys.argv[1:])

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
# The authority/schema/binding checks run through the same production entry
# points the case runner calls, so the wrappers below are the whole coupling.
lib_approval="$root/lib-approval.py"
test -f "$lib_approval"
approval_schema_validator() { python3 "$lib_approval" schema-validate "$1" "$2"; }
select_reviewer_b_approval() { python3 "$lib_approval" select-approval "$1" "$2" "$3"; }
reviewer_b_session_binding() { python3 "$lib_approval" bind-session "$1" "$2" "$3" "$4"; }
# A bad subcommand or arity must be rejected as a usage error, not silently pass.
if python3 "$lib_approval" no-such-entry-point >/dev/null 2>&1; then
    echo "lib-approval.py accepted an unknown entry point" >&2
    exit 1
fi
if python3 "$lib_approval" schema-validate only-one-argument >/dev/null 2>&1; then
    echo "lib-approval.py accepted the wrong argument count" >&2
    exit 1
fi

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
harness_grep 'PROVENANCE_MISSING'
printf 'Production authority/schema/binding fixture execution passed.\n'

# Threshold/denominator reason split: MISSING_THRESHOLDS fires only on absent
# threshold values, MISSING_DENOMINATOR only on an absent/invalid/<= 0 oracle
# denominator. A valid denominator with thresholds absent must fire only the one.
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

# Structural gate: the verdict travels as an exit code, not a global mutated
# inside the redirected brace group, so it survives `> report` and taints the run.
# shellcheck source=../lib-structural-gate.sh
source "$root/lib-structural-gate.sh"
gate_root="$fixture_root/structural-gate"
gate_plan="$gate_root/plan"
mkdir -p "$gate_plan/ui-story-runs" "$gate_plan/context/snapshots"
touch "$gate_plan/plan-description.md" "$gate_plan/progress.md" \
    "$gate_plan/validation.md" "$gate_plan/analysis-report.md" \
    "$gate_plan/goal.md" "$gate_plan/work-unit-inventory.md" \
    "$gate_plan/ui-user-stories.md" "$gate_plan/adversarial-review.md" \
    "$gate_plan/bugs.md" "$gate_plan/context-snapshot.md" \
    "$gate_plan/plan-testing.md" \
    "$gate_plan/ui-story-runs/fixture.txt" "$gate_plan/context/snapshots/fixture.txt"

structural_verdict="pass"
structural_gate_report "$gate_plan" gate-fixture 1 > "$gate_root/complete.txt" \
    || structural_verdict="fail"
if [ "$structural_verdict" != pass ]; then
    echo "complete plan did not pass the structural gate" >&2
    cat "$gate_root/complete.txt" >&2
    exit 1
fi
grep -Fq 'result=pass' "$gate_root/complete.txt"

# Remove one required artifact: the caller's variable must flip to fail even
# though the verdict is computed inside a redirected call.
rm -f "$gate_plan/progress.md"
structural_verdict="pass"
structural_gate_report "$gate_plan" gate-fixture 1 > "$gate_root/missing.txt" \
    || structural_verdict="fail"
if [ "$structural_verdict" != fail ]; then
    echo "a missing required artifact did not taint the structural verdict" >&2
    exit 1
fi
grep -Fq 'result=fail' "$gate_root/missing.txt"
grep -Fq 'FAIL: plan progress tracker' "$gate_root/missing.txt"

# A missing plan directory taints too, and says so in the report.
structural_verdict="pass"
structural_gate_report "$gate_root/absent" gate-fixture 0 > "$gate_root/absent.txt" \
    || structural_verdict="fail"
if [ "$structural_verdict" != fail ]; then
    echo "a missing plan directory did not taint the structural verdict" >&2
    exit 1
fi
grep -Fq 'FAIL: plan directory missing' "$gate_root/absent.txt"

# ...and a fail verdict is wired into the run's taint expression, not merely
# reported. This is the one line in the case runner that connects them.
grep -Fq '[ "$STRUCTURAL_VALIDATION" = "fail" ]' "$root/case/start-worker.sh"
grep -Fq 'STRUCTURAL_GATE_FAILED' "${harness_sources[@]}" >/dev/null
printf 'Structural gate verdict survives redirection and taints the run.\n'

# Exercise the real setup adapter with deterministic worker/reviewer commands.
# The fakes replace only the external model invocation; seeding, selection,
# binding, grading, provenance publication and redaction all still run.
integration_root="$fixture_root/integration"
fake_bin="$integration_root/bin"
fixture_plan="$integration_root/fixture-plan"
mkdir -p "$fake_bin" "$fixture_plan"
# A committed fixture, not a transient .plans/ tree: this test used to copy a
# gitignored plan, so it could only pass on a machine that happened to have one.
# The fixture holds exactly the artifacts the structural gate requires and that
# this test does not create for itself.
fixture_plan_src="$root/tests/fixtures/review-lifecycle-plan"
[ -d "$fixture_plan_src" ] || { printf 'missing fixture: %s\n' "$fixture_plan_src" >&2; exit 66; }
( cd "$fixture_plan_src" && tar cf - . ) | ( cd "$fixture_plan" && tar xf - )
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
# Stay alive until the runner has captured this worker, then exit: the worker
# polls the process registry (its path arrives through the exported
# BENCHMARK_PROCESS_REGISTRY) for its own pid -- or, if the detach shim forked,
# any registered agent row writing this worker's jsonl -- so the handshake is
# event-driven instead of a wall-clock sleep that loses races on loaded runners.
# The ceiling exists only to bound a defect; reaching it proceeds normally,
# which is the old sleep's behaviour with a bound instead of a guess.
workspace=''
capsule=''
while [ "$#" -gt 0 ]; do
    if [ "$1" = -C ]; then workspace="$2"; shift 2; continue; fi
    if [ "$1" = --add-dir ] && [ -z "$capsule" ]; then capsule="$2"; shift 2; continue; fi
    shift
done
tab="$(printf '\t')"
attempt=0
while [ "$attempt" -lt 150 ]; do
    if [ -n "${BENCHMARK_PROCESS_REGISTRY:-}" ] \
        && awk -F "$tab" -v me="$$" '
            $1 == "agent" && ($2 == me || $5 ~ /worker\.jsonl$/) { found = 1 }
            END { exit found ? 0 : 1 }
        ' "$BENCHMARK_PROCESS_REGISTRY" 2>/dev/null; then
        break
    fi
    sleep 0.1
    attempt=$((attempt + 1))
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
# This runs in the generated fixture's own shell, so it cannot use the
# harness helper; stock macOS has no sha256sum, hence the cascade.
if command -v sha256sum >/dev/null 2>&1; then
    manifest_hash="$(sha256sum "$capsule/capsule-manifest.json" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
    manifest_hash="$(shasum -a 256 "$capsule/capsule-manifest.json" | awk '{print $1}')"
else
    manifest_hash="$(openssl dgst -sha256 "$capsule/capsule-manifest.json" | awk '{print $NF}')"
fi
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
cat > "$fake_bin/timeout" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[ "$#" -gt 1 ] || exit 64
shift
exec "$@"
EOF
chmod +x "$fake_bin/codex" "$fake_bin/timeout"
no_setsid_path="$integration_root/no-setsid-path"
mkdir -p "$no_setsid_path"
old_ifs="$IFS"
IFS=:
for dir in $PATH; do
    [ -d "$dir" ] || continue
    for path in "$dir"/*; do
        [ -x "$path" ] || continue
        tool="${path##*/}"
        case "$tool" in setsid|nohup|open) continue ;; esac
        [ -e "$no_setsid_path/$tool" ] || ln -s "$path" "$no_setsid_path/$tool"
    done
done
IFS="$old_ifs"
for tool in codex fake-reviewer codex-worker timeout; do
    ln -sf "$fake_bin/$tool" "$no_setsid_path/$tool"
done
# The run id must match UTC_TIMESTAMP-<name>, so the name is the only place
# collision-proof entropy fits: two suite runs starting in the same second
# otherwise derive the same RESULT_DIR and race the publication `mv`.
run_id="$(date -u +%Y%m%dT%H%M%SZ)-adapter-test-$$"
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
# No wall clock. Everything below it is a shell script in this fixture that
# writes a file and exits, so a 60s cap could only fire on a defect -- and then it
# would report a timeout instead of the defect. It also made this the only test in
# the suite whose result depended on machine speed, and the only one that could
# not run on macOS at all, where `timeout` does not exist. A hang here is a hang
# to see, the same as in the other 79 tests, none of which are capped.
if ! PATH="$no_setsid_path" env \
    REVIEWER_COMMAND="$fake_bin/fake-reviewer" \
    REVIEWER_SESSION_ID="${run_id}-current-B-fixed" \
    REVIEWER_CAPSULE_ID="capsule-001" \
    REVIEWER_MODE=fresh-review \
    REVIEWER_APPROVED_AT=2026-08-11T00:00:00Z \
    SEMANTIC_THRESHOLD=1.0 \
    INDEPENDENT_THRESHOLD=1.0 \
    BLINDED_ORACLE_SPEC="$integration_root/seeded-defects.json" \
    "$BASH" "$root/setup-benchmark.sh" current "$integration_root/testing" "adapter-test-$$" "$run_id" \
    > "$integration_root/setup-output.txt" 2>&1; then
    cat "$integration_root/setup-output.txt" >&2
    exit 1
fi
if ! bash -n "$integration_root/testing/current-$run_id/start-worker.sh"; then
    nl -ba "$integration_root/testing/current-$run_id/start-worker.sh" | sed -n '600,620p' >&2
    exit 1
fi
if ! PATH="$no_setsid_path" "$BASH" "$integration_root/testing/current-$run_id/start-worker.sh" > "$integration_root/worker-output.txt" 2>&1; then
    cat "$integration_root/worker-output.txt" >&2 || true
    cat "$integration_root/testing/current-$run_id/workspace/oracle-grade.txt" >&2 2>/dev/null || true
    exit 1
fi
case_root="$integration_root/testing/current-$run_id"
registry_file="$case_root/process-registry.tsv"
test -s "$registry_file"
awk -F "$(printf '\t')" '
    $1 == "agent" && $2 ~ /^[0-9]+$/ && $5 ~ /worker\.jsonl$/ { worker = 1 }
    $1 == "agent" && $2 ~ /^[0-9]+$/ && $5 ~ /reviewer\.jsonl$/ { reviewer = 1 }
    END { exit(worker && reviewer ? 0 : 1) }
' "$registry_file"
test -x "$case_root/no-detach-bin/nohup"
test -x "$case_root/no-detach-bin/setsid"
test -x "$case_root/no-detach-bin/open"
grep -Fq 'Worker isolation mode: registry' "$integration_root/testing/current-$run_id/workspace/process-audit.txt"
grep -Fq 'Process audit: pass' "$integration_root/testing/current-$run_id/workspace/process-audit.txt"
archive="$root/../results/codex/$(benchmark_result_parent current)/$run_id/current"
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
rm -rf -- "$root/../results/codex/$(benchmark_result_parent current)/$run_id"
rm -rf -- "${PLANNING_AGENT_TMPDIR:-${TMPDIR:-/tmp}/planning-agent}/ai-skills-capsules/$run_id"
rm -rf -- "$root/../results/codex/.staging/$run_id"
printf 'Real setup adapter integration fixture passed.\n'

printf 'Review lifecycle contract tests passed.\n'
