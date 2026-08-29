#!/usr/bin/env python3
"""Reviewer-approval authority, schema, and identity-binding checks.

Extracted verbatim from the quoted `start-worker.sh` heredoc that
`setup-benchmark.sh` used to emit, where these three checks lived as inline
`python3 -` heredocs invisible to every linter and every test.

Each entry point is both importable and callable from the shell:

    lib-approval.py schema-validate  <approval.json> <schema-report.json>
    lib-approval.py select-approval  <reviewer-root> <lifecycle.jsonl> <selection.json>
    lib-approval.py bind-session     <selection.json> <lifecycle.jsonl> <mode> <binding.json>

Contract, unchanged from the heredocs:

  * Every entry point writes its report file even when it fails, because the
    reviewer-state synthesis reads those reports to derive fail-closed reasons.
    A missing report is indistinguishable from a crash, so never return early
    without writing one.
  * Exit 0 means the check passed, 65 means the input data is malformed or the
    binding is rejected (sysexits EX_DATAERR). `select-approval` never fails:
    its verdict is the `selection_reasons` list inside the file it writes.
  * Reviewer B is the only authority. Reviewer A writing an approval.json is
    recorded as UNAUTHORIZED_REVIEWER_APPROVAL, never silently accepted.

Reviewer-evidence contract note (documentation only): a finding's "path" and
"location" fields place its cited evidence using single-file, file-and-section,
or prose/line location forms. `approval_schema_validator` enforces the
structural string requirements; the independent oracle judges which location
form is correct.
"""

import datetime as dt
import hashlib
import json
import pathlib
import sys


def canonical_path_string(path):
    """Return a stable path spelling for equality across symlinked temp roots."""

    try:
        return str(pathlib.Path(path).resolve(strict=False))
    except (OSError, RuntimeError):
        return str(path)


def approval_schema_validator(approval_path, report_path):
    """Validate one reviewer approval envelope; write the schema report.

    Reviewer-evidence contract (documentation only - no behavior change).
    Each finding's "path"/"location" pair records where the cited evidence
    lives, and one of the following location forms is accepted:
      (1) single-file path        - path names the file, location is empty/absent
      (2) file-and-section        - path names the file, location names a
                                    section/heading within that file
      (3) prose/line location     - path names the file, location cites the
                                    prose line(s) or line range within it
    The validator enforces that "path"/"location" are non-empty string fields
    (below) but does not itself judge which location form is correct; that is
    the independent oracle's semantic role.
    """

    approval_path = pathlib.Path(approval_path)
    report_path = pathlib.Path(report_path)
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
    return 0 if not reasons else 65


def select_reviewer_b_approval(reviewer_root, lifecycle_path, selection_path):
    """Pick the single Reviewer B approval and record every rival candidate."""

    reviewer_root = pathlib.Path(reviewer_root)
    lifecycle_path = pathlib.Path(lifecycle_path)
    selection_path = pathlib.Path(selection_path)
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
            roles_by_approval[canonical_path_string(path)] = role

    candidates = []
    unauthorized = []
    paths = sorted(reviewer_root.glob("*/plan/approval.json")) if reviewer_root.is_dir() else []
    for path in paths:
        path_string = str(path)
        role = roles_by_approval.get(path_string) or roles_by_approval.get(canonical_path_string(path))
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
    return 0


def reviewer_b_session_binding(selection_path, lifecycle_path, current_mode, binding_path):
    """Bind the selected approval to one live Reviewer B session, or reject."""

    selection_path = pathlib.Path(selection_path)
    lifecycle_path = pathlib.Path(lifecycle_path)
    binding_path = pathlib.Path(binding_path)
    selection = json.loads(selection_path.read_text(encoding="utf-8"))
    selected = selection.get("selected_approval_path")
    result = {"schema_version": "1.4.2-reviewer-binding", "binding_status": "rejected", "reason_codes": []}
    if not isinstance(selected, str) or not selected:
        result["reason_codes"] = selection.get("selection_reasons") or ["APPROVAL_MISSING"]
        binding_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        return 65

    approval_path = pathlib.Path(selected)
    try:
        approval = json.loads(approval_path.read_text(encoding="utf-8"))
        events = [json.loads(line) for line in lifecycle_path.read_text(encoding="utf-8").splitlines() if line.strip()]
    except (OSError, json.JSONDecodeError):
        result["reason_codes"] = ["REVIEWER_BINDING_EVIDENCE_INVALID"]
        binding_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        return 65

    selected_canonical = canonical_path_string(selected)
    handoffs = [
        event for event in events
        if event.get("event_type") == "handoff"
        and event.get("protocol_role") == "reviewer-b"
        and canonical_path_string(event.get("approval_path", "")) == selected_canonical
    ]
    if len(handoffs) != 1:
        result["reason_codes"] = ["REVIEWER_BINDING_MISSING_EVENT" if not handoffs else "REVIEWER_BINDING_DUPLICATE"]
        binding_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        return 65
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
    return 0 if result["binding_status"] == "passed" else 65


ENTRY_POINTS = {
    "schema-validate": (approval_schema_validator, 2),
    "select-approval": (select_reviewer_b_approval, 3),
    "bind-session": (reviewer_b_session_binding, 4),
}


def main(argv):
    if len(argv) < 2 or argv[1] not in ENTRY_POINTS:
        sys.stderr.write(__doc__.split("Contract")[0].strip() + "\n")
        return 64
    entry, arity = ENTRY_POINTS[argv[1]]
    arguments = argv[2:]
    if len(arguments) != arity:
        sys.stderr.write(f"lib-approval.py: {argv[1]} needs {arity} arguments\n")
        return 64
    return entry(*arguments)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
