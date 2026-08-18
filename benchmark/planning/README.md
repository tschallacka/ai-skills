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

Benchmark **only the current working tree / HEAD** with the special `current`
tag:

```bash
benchmark/planning/run-benchmark.sh current /tmp/ai-skills-benchmark --sequential current
```

`current` is a reserved revision, not a git tag: `setup-benchmark.sh` archives
the live working tree (HEAD, including any uncommitted changes) instead of
checking out a tag. Use it to validate the exact state under test, including
the current commit. It is the smallest possible benchmark (one revision, one
worker case) and is the right choice for a single-commit smoke run.

For a single-commit smoke run of the current working tree, the verbatim
command is:

```bash
BENCHMARK_AGENT=opencode \
resource-limited-testing/scripts/limited-run.sh 6G 400 -- \
  benchmark/planning/run-benchmark.sh smoke-current /tmp/ai-skills-benchmark --sequential current
```

(`OPENCODE_MODEL` selects the model; the driver falls back to its default if
unset. See `AGENTS.md` → "Smoke-test the current state" for the full recipe and
done criterion.)

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

Set the active agent's model environment variable (`CODEX_MODEL` for codex,
`OPENCODE_MODEL` for opencode, `CLAUDE_MODEL` for claude) to select the model
used by workers, reviewers, and the analyzer. Each driver supplies its own
default (`gpt-5.5` for codex) so a stale interactive model selection cannot
block a non-interactive run.

## Benchmark agent runtime

Workers, reviewers, and the analyzer run through `benchmark/planning/runtime/`.
Each agent (`codex` default, `opencode`, `claude`) owns an `agent.sh` driver
implementing one reserved contract (worker/reviewer/analyzer argv builders,
session-id extraction, telemetry with honest unavailable fallback, and model
env/default). `lib-agent.sh` is the single shared launcher for
setsid/timeout/process-group control, and `agent-env.sh` resolves the active
agent. See `runtime/README.md` for the full contract.

Select the active agent with `BENCHMARK_AGENT` (default `codex`):

```bash
BENCHMARK_AGENT=opencode benchmark/planning/setup-benchmark.sh v1.3.0 /tmp/ai-skills-benchmark smoke
BENCHMARK_AGENT=claude benchmark/planning/setup-and-run.sh smoke --sequential
```

The resolved value is persisted into the generated `benchmark-env.sh`, so the
case later runs its worker/reviewers/analyzer and the copied
`telemetry.sh`/`session-id-from-jsonl.sh` under the active agent. Add a new
agent with the scaffolder:

```bash
benchmark/planning/runtime/scaffold-agent.sh <agent> [cli] [model-env] [model-default]
```

then implement the reserved contract in `runtime/<agent>/agent.sh`.
Telemetry is always honest: the active agent's documented store is read, or
`telemetry_status=unavailable:...` is reported instead of a fabricated token
count.

Each run is kept in one timestamped, named directory, scoped by the agent
driver that ran it. For example:

```text
benchmark/results/codex/1.4.1/20260810T111517Z-smoke/
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

Runs are organized as `benchmark/results/<agent>/<revision-parent>/<run-id>/`,
where `<revision-parent>` is the tag being benchmarked (`1.4.1` above). A
`current` run is nested as `benchmark/results/<agent>/current/<latest-tag>/
<run-id>/` so current-state results are grouped under the release that was
latest when they ran.

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
runner/worker processes, `2` for reviewer/agent processes, or `3` for all
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

## The two halves of a case

`setup-benchmark.sh` prepares a case; `case/start-worker.sh` runs it. They used
to be one file: the runner lived inside `setup-benchmark.sh` as a 1270-line
quoted (`<<'EOF'`) heredoc, 84% of the file, invisible to `bash -n`, to
`shellcheck`, and to every test - which is why the lifecycle test used to carve
functions out of it by awk line range. The runner is now a real file that setup
copies verbatim, and the Python and cohesive shell blocks inside it are real
files too:

| File | Owns |
|---|---|
| `case/start-worker.sh` | stage orchestration, `run_reviewer`, the oracle stage |
| `lib-portable.sh` | GNU-vs-BSD primitives: sha256, literal in-place replace, basenames, unique suffix, the `python3` guard |
| `lib-process.sh` | process-group teardown and the post-run browser/server audit |
| `lib-structural-gate.sh` | the required-artifact checks and the five matchers |
| `lib-approval.py` | reviewer authority, approval schema, session binding |
| `synthesize-state.py` | provenance digests and the fail-closed reviewer state |
| `emit-telemetry.py` | `telemetry.json` |

Two contracts are easy to break by accident and are stated in the files
themselves:

- **`benchmark-env.sh` is the only channel.** 30 `printf '%q'`-quoted exports.
  `case/start-worker.sh`'s header enumerates all of them; add or rename one in
  the emitter and that header moves in the same commit.
- **Verdicts travel as return codes, never as shared globals.** The structural
  gate's verdict used to be a variable mutated inside a `{ ... } > report` brace
  group and read afterwards, which is correct only because bash does not fork
  for a redirected group. `structural_gate_report` returns it instead, so the
  caller is right either way.

The case runner's final `exit` is the **worker's** exit code, not the run
verdict: a fully tainted run with a clean worker exits 0. The verdict lives in
`telemetry.json`'s `status` and `reviewer-state.json`'s `adoptable`.

## Files

- `benchmark-test.md`: benchmark procedure and acceptance rules.
- `task-spec.md`: reusable task being planned by each worker.
- `worker-prompt.md`: worker prompt template copied/rendered into each case.
- `analyzer-prompt.md`: analyzer prompt template copied/rendered for the final
  comparison pass.
- `setup-benchmark.sh`: prepares one tag's source/workspace/startup files.
- `case/start-worker.sh`: the case runner, copied verbatim into each case root.
  Its header lists the 30 `benchmark-env.sh` exports it consumes - the single
  channel between the two halves.
- `lib-portable.sh`, `lib-process.sh`, `lib-structural-gate.sh`: sourced by the
  case runner (`lib-portable.sh` also by `setup-benchmark.sh`).
- `lib-approval.py`, `synthesize-state.py`, `emit-telemetry.py`: the case
  runner's Python entry points.
- `run-benchmark.sh`: prepares and runs one or more tags, then starts the
  analyzer.
- `telemetry.sh`: dispatches token accounting to the active agent driver's
  `agent_telemetry`.
- `session-id-from-jsonl.sh`: dispatches session-id extraction to the active
  agent driver's `agent_session_id`.
- For codex, telemetry discovery honors `CODEX_TELEMETRY_DB` when set, then
  scans the active `CODEX_HOME` or `$HOME/.codex` for a database containing the
  worker thread. It supports `threads.tokens_used` stores, log-based usage
  stores, and stored rollout JSONL as a final fallback. opencode reads its
  `opencode.db` session store by session id; claude aggregates the session
  transcript's assistant usage; any driver without a matching record reports
  `telemetry_status=unavailable`.
