# Planning benchmark

Small files for running planning-skill benchmarks from a normal user shell.

## Full batch

```bash
benchmark/planning/run-benchmark.sh /tmp/ai-skills-benchmark
```

Pass tags to limit the batch:

```bash
benchmark/planning/run-benchmark.sh /tmp/ai-skills-benchmark v1.3.0 1.4.1
```

## Single benchmark case

```bash
benchmark/planning/setup-benchmark.sh <tag> <testing-base-dir> [run-id]
```

Example:

```bash
benchmark/planning/setup-benchmark.sh v1.3.0 /tmp/ai-skills-benchmark
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
- `session-id-from-jsonl.sh`: fallback extraction of `thread.started` from
  `worker.jsonl`.
