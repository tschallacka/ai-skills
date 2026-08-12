#!/usr/bin/env bash
# Deterministic per-defect post-run report for a blinded-oracle benchmark run.
#
# Reads the public oracle.json report and prints, for each seeded-defect row:
#   - the defect's neutral ordinal/public label (defect_1 .. defect_n; never a
#     seed ID),
#   - the candidate finding ids considered,
#   - the exact failed predicate(s), or "true positive" when none failed.
#
# Output depends only on the oracle report content: two invocations over the
# same report produce byte-identical output.
#
# Usage:
#   benchmark/planning/post-run-report.sh <oracle.json> > post-run-report.txt

set -euo pipefail

if [ "$#" -ne 1 ]; then
    printf 'Usage: %s <oracle.json>\n' "$(basename "$0")" >&2
    exit 64
fi

oracle_file="$1"
[ -f "$oracle_file" ] || { printf 'oracle report not found: %s\n' "$oracle_file" >&2; exit 66; }

python3 - "$oracle_file" <<'PY'
import json
import sys

oracle_path = sys.argv[1]
try:
    with open(oracle_path, encoding="utf-8") as handle:
        oracle = json.load(handle)
except (OSError, json.JSONDecodeError):
    print("PER-DEFECT POST-RUN REPORT")
    print("error: oracle report is not valid JSON")
    raise SystemExit(0)

per_defect = oracle.get("per_defect")
print("PER-DEFECT POST-RUN REPORT")
if not isinstance(per_defect, list):
    print("per_defect: none")
    raise SystemExit(0)

print("per_defect: %d" % len(per_defect))
print()
for entry in per_defect:
    if not isinstance(entry, dict):
        continue
    index = entry.get("index") if isinstance(entry.get("index"), str) else "defect_?"
    classification = entry.get("classification") if isinstance(entry.get("classification"), str) else "unknown"
    finding_ids = entry.get("finding_ids")
    finding_ids = [fid for fid in finding_ids if isinstance(fid, str)] if isinstance(finding_ids, list) else []
    failed = entry.get("failed_predicates")
    failed = [p for p in failed if isinstance(p, str)] if isinstance(failed, list) else []
    print("[%s]" % index)
    print("  classification       : %s" % classification)
    if finding_ids:
        print("  candidate findings   : %s" % ", ".join(finding_ids))
    else:
        print("  candidate findings   : (none)")
    if failed:
        print("  failed predicates    : %s" % ", ".join(failed))
    else:
        print("  failed predicates    : true positive")
    print()
PY
