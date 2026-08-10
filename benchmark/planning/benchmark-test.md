# Planning benchmark test

Reusable user-run procedure for benchmarking tagged planning skill revisions.
Workers must be launched by the user from a normal shell, not by an
already-sandboxed agent, so Codex can create persisted session state and SQLite
telemetry.

## Entry points

Run all tags:

```bash
benchmark/planning/run-benchmark.sh /tmp/ai-skills-benchmark
```

Run selected tags:

```bash
benchmark/planning/run-benchmark.sh /tmp/ai-skills-benchmark v1.3.0 1.4.1
```

Prepare one case without running it:

```bash
benchmark/planning/setup-benchmark.sh <tag> <testing-base-dir> [run-id]
```

Then run the generated startup script:

```bash
<testing-base-dir>/<revision>/start-worker.sh
```

## Directory layout

Each benchmark batch uses a user-provided testing base directory outside this
repository. Each benchmark test is a subdirectory of that base:

```text
<testing-base-dir>/
├── 1.0.0/
│   ├── source/
│   ├── workspace/
│   │   ├── benchmark-test.md
│   │   ├── task-spec.md
│   │   └── worker-prompt.md
│   ├── benchmark-env.sh
│   ├── telemetry.sh
│   ├── session-id-from-jsonl.sh
│   └── start-worker.sh
└── analysis-<run-id>/
```

`source/` is an archive checkout of the selected tag. `workspace/` is the
worker's current directory. `start-worker.sh` is self-contained for that
benchmark case and can be run directly.

## Setup behavior

`setup-benchmark.sh` performs the per-tag setup:

1. Creates one `<testing-base-dir>/<revision>/` subdirectory.
2. Extracts the tag into `source/`.
3. Copies `task-spec.md` into `source/basic-test-proof-plan.md` when the tag
   does not include the task spec.
4. Copies `benchmark-test.md` and `task-spec.md` into `workspace/`.
5. Renders `worker-prompt.md` from the template.
6. Copies `telemetry.sh` and `session-id-from-jsonl.sh`.
7. Writes `benchmark-env.sh`.
8. Writes executable `start-worker.sh`.

`run-benchmark.sh` calls `setup-benchmark.sh` for every requested tag, runs the
generated startup scripts sequentially, and starts one fresh Codex analyzer
session after all workers finish.

## Required case inputs

- `TAG`: Git tag being benchmarked, such as `v1.3.0` or `1.4.1`.
- `REVISION`: normalized tag with a leading `v` stripped.
- `RUN_ID`: batch label, usually UTC timestamp.
- `CASE_ROOT`: per-revision benchmark directory.
- `SRC_ROOT`: tagged source checkout under `CASE_ROOT/source`.
- `BENCH_ROOT`: worker workspace under `CASE_ROOT/workspace`.
- `PLAN_NAME`: fresh plan directory name under `BENCH_ROOT`.
- `RESULT_DIR`: repository archive directory under
  `benchmark/results/<revision>/<run-id>`.

## Worker startup contract

Each generated `start-worker.sh` must:

1. Run `codex exec` from `BENCH_ROOT` without `--ephemeral`.
2. Capture JSONL output in `workspace/worker.jsonl`.
3. Require or recover `workspace/session-id.txt`.
4. Look up telemetry by that UUID and write `workspace/telemetry.txt`.
5. Run tagged `planning/scripts/validate-plan.sh PLAN_NAME` when available.
6. Audit `BENCH_ROOT` for forbidden HTML/HTM artifacts.
7. Copy the full isolated output to `RESULT_DIR`.
8. Write `RESULT_DIR/evaluation.md`.

Telemetry must be looked up by the UUID in `session-id.txt`; do not use an
inferred most-recent session or a prior report.

## Analyzer contract

After all workers finish, `run-benchmark.sh` starts a fresh analyzer Codex
session from `<testing-base-dir>/analysis-<run-id>`. The analyzer prompt comes
from `analyzer-prompt.md`.

The analyzer may inspect only:

- the copied benchmark instructions in the analysis directory;
- the generated harness summary;
- result archives under `benchmark/results/` whose run id matches the current
  batch.

It must write:

```text
benchmark/results/comparison-<run-id>.md
```

The comparison must include one row per revision and clearly separate worker
exit status, repository-local validation result, HTML/HTM artifact audit,
session UUID, telemetry records, token total or explicit unavailable status,
accepted/tainted benchmark status, and the reason for every tainted result.

The analyzer must not repair worker artifacts or fill missing telemetry from
other sessions.

## Completion checks

- [ ] The user launched the benchmark scripts from a normal shell, not from a
      sandboxed agent tool call.
- [ ] Each benchmark test lives under its own `<testing-base-dir>/<revision>/`
      subdirectory.
- [ ] Each benchmark subdirectory contains its generated `start-worker.sh`.
- [ ] The worker wrote `session-id.txt`, or the startup script recovered the
      first `thread.started` UUID from `worker.jsonl` and recorded that fact.
- [ ] Telemetry lookup used the UUID from `session-id.txt`.
- [ ] The plan directory is new under `BENCH_ROOT` and contains the complete
      durable artifact set.
- [ ] The plan has the intended revision and exact future task contract.
- [ ] The worker used only the tagged `planning/` skill and its relative
      references, not the installed Codex skill.
- [ ] The worker did not inspect previous proof directories, benchmark
      results, repository notes, `.plans/`, git history, or unrelated files.
- [ ] `planning/scripts/validate-plan.sh PLAN_NAME` passes from the isolated
      run when that tag provides the validator.
- [ ] No HTML/HTM artifact was created anywhere in `BENCH_ROOT` or the result
      archive.
- [ ] No browser, server, driver, or unrelated process was started by the run.
- [ ] `benchmark/results/REVISION/RUN_ID/` contains the copied worker output
      and `evaluation.md`.
- [ ] SQLite telemetry is recorded, or its absence is explicitly documented
      and the run is tainted.
- [ ] The analyzer wrote `benchmark/results/comparison-RUN_ID.md`.

## Result template

| Revision | Isolated root | Result archive | Plan | Session ID | Start | End | Elapsed seconds | Usage tokens | Records | Work units | Goals | Validation | Review | Status |
|---|---|---|---|---|---|---|---:|---:|---:|---:|---:|---|---|---|
| `REVISION` | `BENCH_ROOT` | `benchmark/results/REVISION/RUN_ID` | `PLAN_NAME` |  |  |  |  |  |  |  |  |  |  |  |

Runs without persisted telemetry are tainted unless the benchmark purpose
explicitly allows a no-token smoke test.
