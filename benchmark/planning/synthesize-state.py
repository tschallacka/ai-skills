#!/usr/bin/env python3
"""Provenance digests and the fail-closed reviewer-state synthesis.

Extracted from the quoted `start-worker.sh` heredoc `setup-benchmark.sh` used to
emit, where both halves were inline `python3 -` heredocs no linter could see.

    synthesize-state.py provenance <output> <source-plan> <defective-plan> \
        <target-snapshot> <approval> <transcript> <lifecycle> <reviewer-root> \
        <binding> <selection>
    synthesize-state.py state <reviewer-state> <oracle.json> <selection> \
        <schema-report> <binding> <provenance-material>

Ordering is load-bearing and the case runner enforces it: `provenance` must run
before `state`, because `state` reads provenance-material.json to decide whether
PROVENANCE_MISSING fires; and `state` must run before telemetry, which embeds
reviewer-state.json wholesale.

`state` additionally reads five names from the environment rather than argv,
unchanged from the heredoc it came from:

    REVIEWER_STATUS   "passed" or the run is REVIEW_INCOMPLETE
    ORACLE_STATUS     "accepted" | "seeded" | "rejected" | "not-configured"
    STATUS            "tainted" adds TAINTED_RUN; the case runner owns this word
    SEMANTIC_THRESHOLD, INDEPENDENT_THRESHOLD   absent => MISSING_THRESHOLDS

Approval is terminal review evidence, not an adoption decision. The state
calculation is deliberately independent of worker/oracle exit status so a valid
negative approval stays gradeable while adoption fails closed.
"""

import hashlib
import json
import os
import pathlib
import sys


def provenance(output, source_plan, defective_plan, target_snapshot, approval,
               transcript, lifecycle, reviewer_root, binding, selection):
    """Digest every published artifact and record its in-archive path."""
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
    return 0


def state(state_path, oracle_path, selection_path, schema_path, binding_path,
          provenance_path):
    """Derive the fail-closed reviewer/adoption state from the run's evidence."""
    state_path, oracle_path, selection_path, schema_path, binding_path, provenance_path = (
        pathlib.Path(p) for p in (state_path, oracle_path, selection_path, schema_path, binding_path, provenance_path)
    )
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
    return 0


ENTRY_POINTS = {"provenance": (provenance, 10), "state": (state, 6)}


def main(argv):
    if len(argv) < 2 or argv[1] not in ENTRY_POINTS:
        sys.stderr.write(__doc__.split("Ordering")[0].strip() + "\n")
        return 64
    entry, arity = ENTRY_POINTS[argv[1]]
    if len(argv) - 2 != arity:
        sys.stderr.write(f"synthesize-state.py: {argv[1]} needs {arity} arguments\n")
        return 64
    return entry(*argv[2:])


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
