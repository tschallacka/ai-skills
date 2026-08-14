#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
runner="$repo_dir/benchmark/planning/run-benchmark.sh"
setup="$repo_dir/benchmark/planning/setup-benchmark.sh"

grep -Fq 'trap cleanup_on_signal INT TERM' "$runner"
grep -Fq 'kill_process_tree' "$runner"
grep -Fq 'trap cleanup_on_signal INT TERM' "$setup"
grep -Fq 'WORKER_PROCESS_GROUP_ID' "$setup"
grep -Fq 'STAGING_RESULT_DIR' "$setup"
grep -Fq 'copy_workspace_for_publication() {' "$setup"
grep -Fq 'mv "$STAGING_RESULT_DIR" "$RESULT_DIR"' "$setup"
grep -Fq 'STATUS="tainted"' "$setup"
grep -Fq 'No compatibility fallback is permitted' "$setup"
grep -Fq 'refusing compatibility patch' "$setup"
! grep -Fq 'SKIPPED (fixture unavailable' "$setup"
grep -Fq 'telemetry-schema.json' "$repo_dir/benchmark/planning/tests/test-telemetry-integrity.sh"

# The publication boundary is part of the protocol, not merely an archive
# implementation detail. Keep the redaction contract visible in the source
# and fail the suite if a published report can regress to private identifiers.
oracle="$repo_dir/benchmark/planning/review-oracle.sh"
grader="$repo_dir/benchmark/planning/grade-blinded-run.sh"
setup_source="$repo_dir/benchmark/planning/setup-benchmark.sh"
grep -Fq 'redacted' "$oracle"
grep -Fq 'defect_id' "$oracle"
grep -Fq 'mutation' "$oracle"
grep -Fq '<private>' "$oracle"
grep -Fq 'fail_closed_reasons' "$setup_source"
grep -Fq 'adoptable' "$setup_source"
grep -Fq 'oracle.json' "$grader"

if grep -R --line-number --include='*.json' --include='*.md' \
    'ai-skills-oracle-private\|oracle-key\|defect-map.enc' \
    "$repo_dir/benchmark/planning/tests/fixtures" >/dev/null 2>&1; then
    echo 'private oracle material leaked into public fixtures' >&2
    exit 1
fi

# Process-control split after the runtime refactor: the shared launcher
# (lib-agent.sh) owns setsid/timeout/background-mode argv execution for worker,
# reviewer, and analyzer, while the generated start-worker heredoc keeps its own
# trap + WORKER_PROCESS_GROUP_ID + kill_process_tree for worker isolation and
# run-benchmark.sh keeps batch-level cleanup_on_signal. None of the harness
# launch paths may hardcode a bare inline `codex exec` anymore.
runtime="$repo_dir/benchmark/planning/runtime"
grep -Fq 'launch_agent()' "$runtime/lib-agent.sh"
grep -Fq 'cmd+=(setsid)' "$runtime/lib-agent.sh"
grep -Fq 'cmd+=(timeout "$timeout")' "$runtime/lib-agent.sh"
grep -Fq 'background' "$runtime/lib-agent.sh"
grep -Fq 'no setsid' "$runtime/lib-agent.sh"
grep -Fq 'launch_agent setsid "${WORKER_TIMEOUT:-45m}"' "$setup"
grep -Fq 'launch_agent setsid "${REVIEWER_TIMEOUT:-20m}"' "$setup"
grep -Fq 'launch_agent background ""' "$runner"
if grep -Fq 'setsid timeout "${WORKER_TIMEOUT' "$setup" || grep -Fq 'codex -a never exec' "$runner" || grep -Fq 'codex -a never exec' "$setup"; then
    echo 'inline codex exec launch still present in harness source' >&2
    exit 1
fi

printf '%s\n' 'Safeguard contract tests passed.'
