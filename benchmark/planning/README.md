# Planning benchmark

Small files for running planning-skill benchmarks from a normal user shell.

## Full batch

```bash
benchmark/planning/setup-and-run.sh full-batch --sequential
```

Run a named batch through the convenience wrapper. Sequential execution is
the default; use `--parallel` to run worker cases concurrently:

```bash
benchmark/planning/setup-and-run.sh smoke --sequential
benchmark/planning/setup-and-run.sh smoke --parallel
```

The name is required and becomes the suffix of the generated run ID. Run IDs
use the format `UTC_TIMESTAMP-NAME`, and that same name is included in copied
result directories.

Pass tags to limit the batch:

```bash
benchmark/planning/run-benchmark.sh selected /tmp/ai-skills-benchmark --sequential v1.3.0 1.4.1
```

`run-benchmark.sh` requires the run name and testing base directory. It accepts
`--parallel` or `--sequential` immediately afterward. Parallel mode keeps at
most five worker cases active; remaining cases are queued.

For semantic-version tags, each `major.minor` line is benchmarked only once:
the highest patch version is selected (`v1.3.1` supersedes `v1.3.0`).

To interactively choose versions, including bug-fix versions, use:

```bash
benchmark/planning/setup-and-run.sh smoke --sequential --versions
```

The menu accepts multiple numbers such as `1,3`, or `all`.

Pressing Ctrl+C stops the active workers and analyzer, including their child
processes, and exits with status 130.

Set `CODEX_MODEL` to select the model used by workers, reviewers, and the
analyzer; the harness default is `gpt-5.5` so a stale interactive model
selection cannot block a non-interactive run.

Each run is kept in one timestamped, named directory. For example:

```text
benchmark/results/20260810T111517Z-smoke/
├── harness-summary.tsv
├── analysis/
├── comparison.md
├── 1.3.1/
│   ├── evaluation.md
│   ├── planning/
│   └── workspace/
└── 1.4.1/
    ├── evaluation.md
    ├── planning/
    └── workspace/
```

Each revision archive includes the exact tagged `planning/` skill that was
benchmarked, alongside the worker output. This keeps all single-run reports
and revision copies together and makes separate benchmark runs easy to
distinguish.

Protocol 1.4.2 runs are a distinct cohort recorded in
`protocol-metadata.json`. Use `setup-and-run.sh <name> --sequential
--fresh-review` (or explicit `--iterative` and `--revisions tag[,tag...]`).
Legacy archives remain frozen; new capsule inputs and reviewer lifecycle
evidence stay with the new run archive.

The generated `comparison.md` also contains a batch overview, per-revision
deliverable inventory, developer journeys, and a post-hoc token progression
with absolute and percentage changes against the previous usable revision.

### Monitor continuation contract

The monitor must treat a worker/reviewer/analyzer status report, partial
artifact list, unchanged poll, or “I’m working” message as intermediate. While
the subprocess remains active, continue bounded polling and steer it with an
explicit next action. Before each steering action, inspect bounded process
state, latest output, expected artifacts, elapsed time, and retry budget.

Stop only on terminal evidence: process exit with a result, an accepted/
tainted/rejected archive, a validated completion report, or a recorded blocker
after the retry budget is exhausted. Preserve the last output, process audit,
next action, steering/retry count, and terminal reason. Do not restart blindly
or report success from a status-only message.

For repeated long checks, use an executable `/tmp` helper with the contract
`helper PROFILE RUN_ID CASE_ROOT RESULT_ROOT`. `PROFILE` must be `1` for
runner/worker processes, `2` for reviewer/Codex processes, or `3` for all
in-scope processes; unsupported values fail with exit code 64. Output must be
bounded and the helper must be removed when it is run-specific.

Historical planning tests that reference a developer-only context fixture
require `PLANNING_CONTEXT_CACHE` to be set to an existing fixture directory.
Missing configuration or an unavailable fixture is an error; the tests do not
fall back to another user's home directory or silently skip required coverage.

## Benchmark Python environment

The benchmark folder contains a Nix flake with Python and PyYAML for Python
validation tools such as `quick_validate.py`:

```bash
cd benchmark
nix develop
python3 ~/.codex/skills/.system/skill-creator/scripts/quick_validate.py planning
```

This environment is benchmark tooling only; the planning skill itself remains
shell-based and does not require Python.

## Single benchmark case

```bash
benchmark/planning/setup-benchmark.sh <tag> <testing-base-dir> <name> [run-id]
```

Example:

```bash
benchmark/planning/setup-benchmark.sh v1.3.0 /tmp/ai-skills-benchmark smoke
/tmp/ai-skills-benchmark/1.3.0/start-worker.sh
```

## Files

- `benchmark-test.md`: benchmark procedure and acceptance rules.
- `task-spec.md`: reusable task being planned by each worker.
- `worker-prompt.md`: worker prompt template copied/rendered into each case.
- `analyzer-prompt.md`: analyzer prompt template copied/rendered for the final
  comparison pass.
- `setup-benchmark.sh`: prepares one tag's source/workspace/startup files.
- `run-benchmark.sh`: prepares and runs one or more tags, then starts the
  analyzer.
- `telemetry.sh`: reads Codex SQLite telemetry by `THREAD_ID`.
- Telemetry discovery honors `CODEX_TELEMETRY_DB` when set, then scans the
  active `CODEX_HOME` or `$HOME/.codex` for a database containing the worker
  thread. It supports `threads.tokens_used` stores, log-based usage stores,
  and stored rollout JSONL as a final fallback.
- `session-id-from-jsonl.sh`: fallback extraction of `thread.started` from
  `worker.jsonl`.
