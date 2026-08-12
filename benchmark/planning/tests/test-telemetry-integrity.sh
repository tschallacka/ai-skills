#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
schema="$root/../../planning/telemetry-schema.json"
python3 -m json.tool "$schema" >/dev/null
grep -Fq 'protocol_id' "$schema"
grep -Fq 'taint_causes' "$schema"
grep -Fq 'reviewer_records' "$schema"
grep -Fq 'telemetry.json' "$root/setup-benchmark.sh"
python3 - <<'PY'
import json
from pathlib import Path
schema = json.loads(Path("benchmark/planning/telemetry-schema.json").read_text())
required = set(schema["required"])
assert {"protocol_id", "taint_causes", "provenance", "phase_records"} <= required
PY
test -x "$root/validate-telemetry.sh"
if "$root/telemetry.sh" 'bad id with spaces' | grep -Fq 'telemetry_status=available'; then
    echo 'invalid telemetry identity was accepted' >&2
    exit 1
fi
printf 'Telemetry integrity contract tests passed.\n'
