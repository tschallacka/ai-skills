#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
setup="$root/setup-benchmark.sh"
grep -Fq 'CAPSULE_BASE="${PLANNING_AGENT_TMPDIR:-${TMPDIR:-/tmp}/planning-agent}/ai-skills-capsules"' "$setup"
grep -Fq 'CAPSULE_ROOT="$CAPSULE_BASE/$RUN_ID/$REVISION/worker"' "$setup"
grep -Fq 'worker-manifest.json' "$setup"
grep -Fq -- '--add-dir "$WORKER_CAPSULE"' "$setup"
grep -Fq -- '--add-dir "$WORKER_WORKSPACE"' "$setup"
if grep -Fq -- '--add-dir "$SRC_ROOT"' "$setup"; then
    echo 'worker launch still exposes SRC_ROOT' >&2
    exit 1
fi
grep -Fq 'Do not inspect `SRC_ROOT`' "$root/worker-prompt.md"
grep -Fq 'only its run instructions, harness summary, current run' "$root/analyzer-prompt.md"
printf 'Capsule access contract tests passed.\n'
