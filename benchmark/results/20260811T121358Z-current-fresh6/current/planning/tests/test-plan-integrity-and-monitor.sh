#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
skill="$root/planning/SKILL.md"
readme="$root/benchmark/planning/README.md"

grep -Fq 'Dynamic scope additions are plan mutations' "$skill"
grep -Fq 'journey entry alone is not a valid plan addition' "$skill"
grep -Fq 'Persistent monitor steering' "$skill"
grep -Fq 'Stop only on terminal evidence' "$skill"
grep -Fq 'bounded selector flag' "$skill"
grep -Fq 'Monitor continuation contract' "$readme"
grep -Fq 'helper PROFILE RUN_ID CASE_ROOT RESULT_ROOT' "$readme"
grep -Fq 'unsupported values fail with exit code 64' "$readme"

printf 'Plan integrity and monitor contract test passed.\n'
