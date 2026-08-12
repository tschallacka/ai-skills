#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
    printf 'Usage: %s <schema.json> <telemetry.json>\n' "$(basename "$0")" >&2
    exit 64
fi

python3 - "$1" "$2" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    schema = json.load(handle)
with open(sys.argv[2], encoding="utf-8") as handle:
    document = json.load(handle)

for key in schema.get("required", []):
    if key not in document:
        raise SystemExit(f"missing required telemetry field: {key}")
for key, expected in {
    "schema_version": "1.4.2",
    "protocol_id": "reviewer-optimization-1.4.2",
}.items():
    if document.get(key) != expected:
        raise SystemExit(f"invalid telemetry {key}: {document.get(key)!r}")
if document.get("reviewer_mode") not in {"fresh-review", "iterative"}:
    raise SystemExit("invalid reviewer_mode")
if document.get("status") not in {"accepted", "tainted", "rejected", "interrupted"}:
    raise SystemExit("invalid telemetry status")
if not isinstance(document.get("taint_causes"), list):
    raise SystemExit("taint_causes must be an array")
if not isinstance(document.get("phase_records"), list):
    raise SystemExit("phase_records must be an array")
print("telemetry schema validation passed")
PY
