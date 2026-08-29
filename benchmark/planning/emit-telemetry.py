#!/usr/bin/env python3
"""Build one benchmark case's telemetry.json on stdout.

Extracted from the quoted `start-worker.sh` heredoc `setup-benchmark.sh` used to
emit. Takes no arguments: every input is an exported environment variable, which
is how the heredoc worked and is kept unchanged so the case runner's single
`export` line stays the whole interface.

    emit-telemetry.py > telemetry.json

Required in the environment (the case runner exports all of them together):

    PROTOCOL_ID RUN_ID REVISION SESSION_ID STATUS REVIEW_MODE START END ELAPSED
    CODE VALIDATION STRUCTURAL_VALIDATION PROCESS_AUDIT HTML_COUNT
    REVIEWER_STATUS TELEMETRY_SOURCE TELEMETRY_STATUS TOTAL_USAGE USAGE_RECORDS
    REVIEWER_LIFECYCLE ORACLE_STATUS REVIEWER_STATE

Ordering is load-bearing: REVIEWER_STATE must already name a written
reviewer-state.json (synthesize-state.py state), because that file is embedded
wholesale as `review_state` and read for `per_defect` and `provenance`.

`status` is copied from STATUS verbatim - this script never decides the verdict,
it only records it, and derives the human-readable `taint_causes` list from the
same per-layer variables the case runner used to compute STATUS. One artifact
never substitutes an inferred total: an unparsable TOTAL_USAGE becomes null with
`token_total_precision: "unavailable"`.
"""

import json
import os
import sys


def main():
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
    sys.stdout.write(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
