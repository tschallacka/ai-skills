Run a fresh isolated planning-only basic test proof for the repository-local
planning skill revision {{REVISION}}.

Your current working directory is {{BENCH_ROOT}}. Treat this directory as your
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
{{SRC_ROOT}}/basic-test-proof-plan.md
{{SRC_ROOT}}/planning/SKILL.md
```

Read `benchmark-test.md`, `task-spec.md`, the task specification in
`{{SRC_ROOT}}/basic-test-proof-plan.md`, and the repository-local planning
skill in full. When `planning/SKILL.md` names another reference, resolve that
path relative to `{{SRC_ROOT}}/planning/` and read the resolved
repository-local file. Do not read or use the installed skill at
`/home/tschallacka/.codex/skills/planning`.

Create a fresh plan under:

```text
{{PLAN_NAME}}
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

Record exact start/end timestamps, elapsed time, worker result, validation
results, review result, artifact/process audit, thread ID, and token usage.

Before completion, audit only the isolated benchmark workspace for generated
artifacts and the expected plan output. Do not audit the repository except for
the exact allowed source paths above.

End with a concise execution summary and the exact path to the analysis report.
