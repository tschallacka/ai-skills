Run a fresh isolated planning-only basic test proof for the repository-local
planning skill revision 1.4.1.

Your current working directory is /tmp/20260810T121526Z-benchmark8/1.4.1/workspace. Treat this directory as your
whole benchmark workspace. At startup it should contain `benchmark-test.md`,
`task-spec.md`, and `worker-prompt.md`, plus any Codex runtime metadata created
by the runner.

Immediately write your session UUID to:

```text
session-id.txt
```

Use `CODEX_THREAD_ID` when it is available. If `CODEX_THREAD_ID` is
unavailable, write the best session UUID exposed by the runner environment and
state the source in the analysis report.

Use only this tagged repository-local skill as the benchmark target:

```text
/tmp/20260810T121526Z-benchmark8/1.4.1/source/basic-test-proof-plan.md
/tmp/20260810T121526Z-benchmark8/1.4.1/source/planning/SKILL.md
```

Read `benchmark-test.md`, `task-spec.md`, the task specification in
`/tmp/20260810T121526Z-benchmark8/1.4.1/source/basic-test-proof-plan.md`, and the repository-local planning
skill in full. When `planning/SKILL.md` names another reference, resolve that
path relative to `/tmp/20260810T121526Z-benchmark8/1.4.1/source/planning/` and read the resolved
repository-local file. Do not read or use any installed planning skill outside
the tagged source paths above.

Create a fresh plan under:

```text
basic-test-proof-1.4.1-20260810T121526Z-benchmark8-isolated-plan
```

The future task is:

- Create `button-chain.html` with one initial button.
- Pressing the current last button appends exactly one button below it.
- Pressing the fourth generated button clears the document.
- The completion state prints the exact lowercase text `finished` with a
  visible white border.

This is a planning-only proof. Do not create, edit, open, inspect, serve, or
test any HTML. Do not start a browser, server, driver, or other execution
tooling.

Use normal runner behavior. Do not force sequential execution, disable
parallelism, disable subagents, or use ephemeral mode. Preserve the full Codex
SQLite telemetry for this run.

Complete the decomposition, atomic work-unit inventory, UI user story, UI story
run/cache, testing companions, adversarial review, bug register, progress
trackers, context snapshot, validation, and analysis report.

The following files are mandatory deliverables inside the actual plan directory
(not merely in the workspace root):

```text
plan-description.md
progress.md
validation.md
analysis-report.md
<at least one goal.md>
<a work-unit inventory>
<a UI user-story document>
ui-story-runs/<story>.md
<at least one *-testing.md companion>
adversarial-review.md
<a bug register such as bug-register.md or bugs.md>
<a context snapshot such as context-snapshot.md or context/snapshots/>
```

Prefer these exact canonical names: `validation.md`, `analysis-report.md`,
`context-snapshot.md`, and `ui-story-runs/<story>.md`. Do not create empty
placeholder files. Each report must contain the actual result and evidence.
If the tagged skill places the plan under `.plans/`, treat that directory as
the plan directory and put all mandatory files there.

Before saying the work is complete, run the tagged validator one final time
and save its output into the plan itself. Use the equivalent of:

```bash
validator="/tmp/20260810T121526Z-benchmark8/1.4.1/source/planning/scripts/validate-plan.sh"
plan_dir="basic-test-proof-1.4.1-20260810T121526Z-benchmark8-isolated-plan"
if [ -d ".plans/basic-test-proof-1.4.1-20260810T121526Z-benchmark8-isolated-plan" ]; then
  plan_dir=".plans/basic-test-proof-1.4.1-20260810T121526Z-benchmark8-isolated-plan"
fi
set +e
validator_output="$($validator "$plan_dir" 2>&1)"
validator_code="$?"
set -e
{
  printf '%s\n' '# Validation report' ''
  printf 'Validator: %s\nExit code: %s\n\n' "$validator" "$validator_code"
  printf '%s\n' "$validator_output"
} > "$plan_dir/validation.md"
test "$validator_code" -eq 0
```

Then inspect the plan directory and verify every mandatory artifact above is a
non-empty file or, for `context/snapshots/`, a non-empty directory. If any
artifact is missing, create the substantive artifact before completing.

Record exact start/end timestamps, elapsed time, worker result, validation
results, review result, artifact/process audit, thread ID, and token usage.

Before completion, audit only the isolated benchmark workspace for generated
artifacts and the expected plan output. Do not audit the repository except for
the exact allowed source paths above.

End with a concise execution summary and the exact path to the analysis report.
