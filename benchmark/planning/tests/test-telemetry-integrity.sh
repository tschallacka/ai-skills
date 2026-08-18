#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
schema="$root/../../planning/telemetry-schema.json"
python3 -m json.tool "$schema" >/dev/null
grep -Fq 'protocol_id' "$schema"
grep -Fq 'taint_causes' "$schema"
grep -Fq 'reviewer_records' "$schema"
# The generated-case source spans setup-benchmark.sh *and* the extracted
# benchmark/planning/case/*.sh, so harness-wide assertions search both, matching
# the harness_grep idiom in test-safeguards.sh.
harness_sources=("$root/setup-benchmark.sh")
for case_source in "$root/case"/*.sh; do
    [ -f "$case_source" ] && harness_sources+=("$case_source")
done
harness_grep() { grep -Fq -- "$1" "${harness_sources[@]}"; }
harness_grep 'telemetry.json'
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
