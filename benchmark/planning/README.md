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

The generated `comparison.md` also contains a batch overview, per-revision
deliverable inventory, developer journeys, and a post-hoc token progression
with absolute and percentage changes against the previous usable revision.

Historical planning tests that reference a developer-only context fixture are
made portable during setup. Set `PLANNING_CONTEXT_CACHE` to run that fixture;
otherwise the test records an explicit skip instead of failing on a hardcoded
home directory.

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
