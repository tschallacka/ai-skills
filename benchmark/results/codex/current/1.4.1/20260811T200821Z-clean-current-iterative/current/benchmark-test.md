# Planning benchmark test

Reusable user-run procedure for benchmarking tagged planning skill revisions.
Workers must be launched by the user from a normal shell, not by an
already-sandboxed agent, so Codex can create persisted session state and SQLite
telemetry.

## Entry points

Run all tags with a required name:

```bash
benchmark/planning/setup-and-run.sh full-batch --sequential
```

Run selected tags:

```bash
benchmark/planning/run-benchmark.sh selected /tmp/ai-skills-benchmark --sequential v1.3.0 1.4.1
```

Run a named batch with explicit worker scheduling:

```bash
benchmark/planning/setup-and-run.sh <name> --sequential
benchmark/planning/setup-and-run.sh <name> --parallel
```

`<name>` is required. Sequential execution is the default when the mode is
omitted. The mode can also be passed to `run-benchmark.sh` immediately after
the testing base directory. Parallel mode keeps at most five worker cases
active and queues the rest.

Prepare one case without running it:

```bash
benchmark/planning/setup-benchmark.sh <tag> <testing-base-dir> <name> [run-id]
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
generated startup scripts sequentially or in parallel according to the
selected mode, and starts one fresh Codex analyzer session after all workers
finish.

Before setup, semantic-version tags are grouped by `major.minor` and only the
highest patch version in each group is retained. For example, `v1.3.1` is run
and `v1.3.0` is skipped. Non-semantic-version tags remain eligible.

Use `--versions` for an interactive multi-select menu of every available tag.
This mode does not apply patch-version deduplication, so both `v1.3.0` and
`v1.3.1` can be selected.

The runner must trap `INT` and `TERM`, terminate active worker/analyzer process
trees, and exit 130 on an interrupted run. A run must not leave benchmark
workers, Codex sessions, browsers, servers, or drivers running after Ctrl+C.

Setup also rewrites known historical tests that reference a developer-specific
planning-context fixture. `PLANNING_CONTEXT_CACHE` can provide that fixture;
when it is unavailable, the test emits a recorded skip rather than causing a
portable benchmark worker failure.

## Required case inputs

- `TAG`: Git tag being benchmarked, such as `v1.3.0` or `1.4.1`.
- `REVISION`: normalized tag with a leading `v` stripped.
- `RUN_ID`: batch label in `UTC_TIMESTAMP-NAME` form for named runs.
- `CASE_ROOT`: per-revision benchmark directory.
- `SRC_ROOT`: tagged source checkout under `CASE_ROOT/source`.
- `BENCH_ROOT`: worker workspace under `CASE_ROOT/workspace`.
- `PLAN_NAME`: fresh plan directory name under `BENCH_ROOT`.
- `RESULT_DIR`: repository archive directory under the run directory
  `benchmark/results/<UTC_TIMESTAMP>-<name>/<revision>`.

## Worker startup contract

Each generated `start-worker.sh` must:

1. Run `codex exec` from `BENCH_ROOT` without `--ephemeral`.
2. Capture JSONL output in `workspace/worker.jsonl`.
3. Require or recover `workspace/session-id.txt`.
4. Look up telemetry by that UUID and write `workspace/telemetry.txt`.
5. Run tagged `planning/scripts/validate-plan.sh PLAN_NAME` when available.
6. Run the harness structural artifact gate for the required plan deliverables.
7. Run the worker in its own process group and audit only matching
   browser/server/driver processes remaining in that group.
8. Audit `BENCH_ROOT` for forbidden HTML/HTM artifacts.
9. Copy the full isolated output to `RESULT_DIR`.
10. Copy the exact tagged `SRC_ROOT/planning/` skill to
    `RESULT_DIR/planning/` for provenance.
11. Write `RESULT_DIR/evaluation.md`.

Telemetry must be looked up by the UUID in `session-id.txt`; do not use an
inferred most-recent session or a prior report.

## Analyzer contract

After all workers finish, `run-benchmark.sh` starts a fresh analyzer Codex
session from `<testing-base-dir>/analysis-<run-id>`. The analyzer prompt comes
from `analyzer-prompt.md`.

The analyzer may inspect only:

- the copied benchmark instructions in the analysis directory;
- the generated harness summary;
- the current run directory under `benchmark/results/<UTC_TIMESTAMP>-<name>/`.

It must write:

```text
benchmark/results/<run-id>/comparison.md
```

The comparison must include one row per revision and clearly separate worker
exit status, repository-local validation result, HTML/HTM artifact audit,
session UUID, telemetry records, token total or explicit unavailable status,
accepted/tainted benchmark status, and the reason for every tainted result.
It must begin with a batch overview containing the total revision count,
successful count, tainted count, and unusable-telemetry count. It must include
a per-revision deliverable inventory with observable counts for goals,
work-unit items, UI story/run-cache items, testing companions, review reports,
bug-register items, context snapshots, validation/analysis reports, plan files,
result files, telemetry records, and tokens. Ambiguous review or fix counts
must be marked `not recorded` rather than guessed.
It must also include a post-hoc token progression comparing each revision with
the previous usable-token revision, including absolute and percentage deltas,
and a cautious interpretation of likely planning-work expansion or reduction.
Observed token changes must not be presented as causal proof.
It must also include a short per-revision developer-journey summary based on
observable worker actions and artifacts: approach, review rounds, correction or
fix cycles, final validation, and notable constraints. It must not claim access
to private chain-of-thought; unknown counts must be marked as not recorded.

All post-run counts must use the selected plan directory only. If more than one
candidate plan directory exists, the analyzer must report the extra directory
as an integrity warning and must not merge its files into the selected plan's
counts. Work-unit counts must count ID rows in the work-unit inventory, not
inventory filenames. Review rounds and fix cycles may be counted only when the
artifacts or event sequence establish their boundaries; otherwise they are
`not recorded`. Token totals and progression comparisons must use only the
UUID-matched telemetry recorded by the worker.

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
- [ ] There is exactly one selected plan directory; any additional plan
      directory is reported as an integrity warning and excluded from counts.
- [ ] The plan has the intended revision and exact future task contract.
- [ ] The worker used only the tagged `planning/` skill and its relative
      references, not the installed Codex skill.
- [ ] The worker did not inspect previous proof directories, benchmark
      results, repository notes, `.plans/`, git history, or unrelated files.
- [ ] `planning/scripts/validate-plan.sh PLAN_NAME` passes from the isolated
      run when that tag provides the validator.
- [ ] The harness structural artifact gate passes for the plan description,
      trackers, goals, work-unit inventory, UI story/cache, testing companion,
      adversarial review, bug register, context, validation, and analysis
      report.
- [ ] The plan itself contains a substantive `validation.md` produced by the
      final tagged validator run; `harness-validation.txt` alone is not a
      substitute.
- [ ] No HTML/HTM artifact was created anywhere in `BENCH_ROOT` or the result
      archive.
- [ ] The worker-owned process-group audit finds no matching browser, server,
      or driver process remaining after the worker; unrelated host processes
      and other parallel workers are excluded.
- [ ] `benchmark/results/RUN_ID/REVISION/` contains the copied worker output
      and `evaluation.md`, plus the exact tagged benchmarked skill under
      `benchmark/results/RUN_ID/REVISION/planning/`.
- [ ] The archived `planning/` skill is sourced from the tagged
      `SRC_ROOT/planning/` directory and its revision is recorded in
      `evaluation.md`.
- [ ] Telemetry is recorded by discovering the Codex SQLite store under
      `CODEX_HOME`/`HOME` and matching the worker UUID, or its absence is
      explicitly documented and the run is tainted.
- [ ] The analyzer exited successfully and wrote a non-empty
      `benchmark/results/RUN_ID/comparison.md`.

## Result template

| Revision | Isolated root | Result archive | Plan | Session ID | Start | End | Elapsed seconds | Usage tokens | Records | Work units | Goals | Validation | Review | Status |
|---|---|---|---|---|---|---|---:|---:|---:|---:|---:|---|---|---|
| `REVISION` | `BENCH_ROOT` | `benchmark/results/RUN_ID/REVISION` | `PLAN_NAME` |  |  |  |  |  |  |  |  |  |  |  |

Runs without persisted telemetry are tainted unless the benchmark purpose
explicitly allows a no-token smoke test.

## 1.4.2 reviewer protocol

The runner accepts `--fresh-review` (default), explicit `--iterative`, and
`--revisions tag[,tag...]`. Iterative mode is bounded to three verification
passes per reviewer and three fresh-review cycles. Reviewer A owns only its
stable findings and cannot approve the overall plan; Reviewer B is a fresh
session/capsule with no A conclusions and performs the mandatory final
independent approval. Malformed combinations fail with exit 64 before setup.
