#!/usr/bin/env bash
# MODE: DEV
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
# Report the failing expression: these assertions are bare `[ ... ]` under
# set -e, which otherwise exits 1 in silence.
t_trap_assertions


root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
skill="$root/planning/SKILL.md"
readme="$root/benchmark/planning/README.md"
monitor_sh="$root/planning/scripts/monitor-read.sh"

# Contract language must be present in the docs (doc-guard; behavior asserted below).
grep -Fq 'Dynamic scope additions are plan mutations' "$skill"
grep -Fq 'journey entry alone is not a valid plan addition' "$skill"
grep -Fq 'Persistent monitor steering' "$skill"
grep -Fq 'Stop only on terminal evidence' "$skill"
grep -Fq 'bounded selector flag' "$skill"
grep -Fq 'Monitor continuation contract' "$readme"
grep -Fq 'helper PROFILE RUN_ID CASE_ROOT RESULT_ROOT' "$readme"
grep -Fq 'unsupported values fail with exit code 64' "$readme"

# Functional: the documented "unsupported values fail with exit code 64" and the
# monitor's fail-closed identity must actually hold, not just be written down.
rc=0
if "$BASH" "$monitor_sh" bogus-subcommand >/dev/null 2>&1; then rc=0; else rc=$?; fi
[ "$rc" -eq 64 ] || { echo "FAIL: monitor-read unknown subcommand did not exit 64 (got $rc)" >&2; exit 1; }
rc=0
if "$BASH" "$monitor_sh" show /nonexistent-frame >/dev/null 2>&1; then rc=0; else rc=$?; fi
[ "$rc" -eq 64 ] || { echo "FAIL: monitor-read without maintainer identity did not fail closed (got $rc)" >&2; exit 1; }

printf 'Plan integrity and monitor contract test passed.\n'
