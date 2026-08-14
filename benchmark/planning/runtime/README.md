# Benchmark agent runtime

`benchmark/planning/runtime/` makes the planning benchmark harness
agent-agnostic. Each supported CLI (currently `codex`, `opencode`, `claude`)
owns a driver folder with an `agent.sh` implementing one reserved contract. A
shared launcher owns all process control, and the active agent is resolved by
`BENCHMARK_AGENT` (default `codex`).

## Layout

```text
runtime/
├── README.md            this file
├── lib-agent.sh         shared launcher + argv machinery (source this)
├── agent-env.sh         active-agent resolver (BENCHMARK_AGENT -> AGENT_DRIVER)
├── scaffold-agent.sh    first-time driver scaffold from TEMPLATE/
├── TEMPLATE/agent.sh    reserved-contract template (failing stubs)
├── codex/agent.sh       Codex driver (default)
├── opencode/agent.sh    opencode driver
└── claude/agent.sh      Claude Code driver
```

## Reserved driver contract

Sourcing a driver must define exactly these symbols. Functions set or print
data; they must not launch the agent themselves — `launch_agent` in
`lib-agent.sh` is the only process-control path.

| Symbol | Kind | Contract |
|---|---|---|
| `AGENT_NAME` | variable | agent id, e.g. `codex`. |
| `agent_bin` | variable | CLI name the driver puts at the front of `AGENT_ARGV`, e.g. `codex`. Used by the reviewer-presence gate (`command -v "$agent_bin"`). |
| `agent_model_env` | variable | environment variable that overrides the model (e.g. `CODEX_MODEL`). |
| `agent_default_model` | variable | model string used when `agent_model_env` is unset. |
| `agent_argv_worker <workspace> <capsule> <prompt>` | function | fills `AGENT_ARGV` for a worker run: `cwd=<workspace>`, capsule/workspace made readable, `<prompt>` content as the message. |
| `agent_argv_reviewer <workspace> <capsule> <prompt> [binary]` | function | fills `AGENT_ARGV` for a Reviewer A/B run. Optional `$4` is the `REVIEWER_COMMAND` test seam: it replaces the driver binary (argv[0]) rather than appending codex flags. For a non-codex driver the seam is documented codex-only; when set it must not leak invalid flags. |
| `agent_argv_analyzer <workspace> <capsule> <prompt>` | function | fills `AGENT_ARGV` for the batch analyzer run. |
| `agent_session_id <agent-output.jsonl>` | function | prints the session id extracted from the agent's output stream, or empty for an honest degrade. |
| `agent_telemetry <session-id>` | function | prints `thread_id`, `usage_records`, `total_usage_tokens`, `telemetry_status` (plus optional `telemetry_db`/`telemetry_source`). Never fabricate token numbers; when no documented store exists for the agent print `telemetry_status=unavailable:...`. |

The resolved model for the active driver is `"${!agent_model_env:-agent_default_model}"`;
`agent_resolve_model` (from `lib-agent.sh`) prints it. `agent_argv_*` functions
must call it so each role honours `CODEX_MODEL`/`OPENCODE_MODEL`/`CLAUDE_MODEL`.

## Active-agent resolution

`agent-env.sh` `resolve_active_agent <runtime_dir>` exports `AGENT_DRIVER`:

1. `BENCHMARK_AGENT` set and a driver exists under `runtime/<agent>/` → selected.
2. `BENCHMARK_AGENT` set to an unknown agent → fail closed (exit 64).
3. `BENCHMARK_AGENT` unset → `codex` (the unchanged default harness path and
   test integration need no env).
4. No `codex` driver → best-effort detection: if exactly one installed CLI
   matches a present driver, select it; otherwise fail closed (exit 64).

`BENCHMARK_AGENT` is persisted into each generated case's `benchmark-env.sh`
at setup time (resolved through `agent-env.sh`, defaulting to `codex`), so
`start-worker.sh` and the copied `telemetry.sh`/`session-id-from-jsonl.sh`
honour the active agent when the case runs later.

## Shared launcher (`lib-agent.sh`)

Sourcing `lib-agent.sh` resolves `RUNTIME_DIR` from its own location, sources
`agent-env.sh`, resolves `AGENT_DRIVER`, sources the driver, and exports:

- `launch_agent <mode> <timeout> <output|'-'>` — runs `${AGENT_ARGV[@]}`
  (set by a driver argv function first) in the background, then records
  `AGENT_PID` and (for `setsid` mode) `AGENT_PGID`.
  - `mode=setsid` wraps with `setsid` and the given `timeout` (e.g. `45m`);
    this is used by workers and reviewers.
  - `mode=background` is a plain background run with **no setsid and no
    timeout**; it preserves the pre-refactor analyzer semantics exactly.
- `wait_agent` — waits for `AGENT_PID` and stores the exit code in
  `AGENT_EXIT`.
- `kill_process_tree <pid> <signal>` — recursive child-tree kill.
- `agent_available` — 0 when `command -v "$agent_bin"` succeeds (the
  reviewer-presence gate).
- `resolve_rep_root` — prints the repo root that owns this runtime.

Consumers resolve the runtime via `REPO_ROOT` (exported by `benchmark-env.sh`
for generated case scripts) with a `dirname` fallback when a script is invoked
from the repo checkout directly (e.g. `test-telemetry-integrity.sh`).

### Process-control split

- The generated `start-worker.sh` keeps its own `trap` + `WORKER_PROCESS_GROUP_ID`
  + `kill_process_tree` for worker isolation and the post-run process audit.
- `lib-agent.sh` owns the shared setsid/timeout/mode launch used to run the
  argv (`launch_agent`).
- `run-benchmark.sh` keeps batch-level `cleanup_on_signal`/`kill_process_tree`
  and uses the analyzer PID reported by the launcher.

### Codex-only extraction that degrades for other agents

Worker-internal subagent extraction (`spawn_agent`/`close_agent` JSONL events
in `start-worker.sh`) and the codex-shaped JSONL session-id fallback are
codex-only. For `opencode`/`claude` those paths read a different stream and
degrade to no-events / empty session id without error; their drivers implement
their own `agent_session_id`.

## Adding an agent

1. Scaffold from the template:

   ```bash
   benchmark/planning/runtime/scaffold-agent.sh <agent> [cli] [model-env] [model-default]
   # e.g. scaffold-agent.sh acme acme ACME_MODEL acme-5
   ```

   This writes `runtime/<agent>/agent.sh` and refuses to clobber an existing
   driver. Re-running for the same name fails without changing anything
   (idempotent/no-clobber).

2. Implement every reserved function in the new `runtime/<agent>/agent.sh`
   (see the contract table above), including the model env/default pair.
3. Give the agent real capsule/workspace readability through its native
   mechanism (e.g. codex `--add-dir`, claude `--add-dir`, opencode
   `--dir` + `-f` attachments / `--auto`). Do not fake a normalized flag
   layer; record the mechanism in the driver.
4. If the agent has a documented telemetry store, implement
   `agent_telemetry`; otherwise keep the honest `unavailable` fallback.
5. Run a real worker invocation (like the `W13`/`W15` proofs) and record
   session id + telemetry status.
6. Set `BENCHMARK_AGENT=<agent>` when running setup:

   ```bash
   BENCHMARK_AGENT=<agent> benchmark/planning/setup-benchmark.sh <tag> <dir> <name>
   ```

## First-time setup

The default harness path is unchanged: with `BENCHMARK_AGENT` unset, `codex`
is used and no runtime configuration is required. To select another agent,
export `BENCHMARK_AGENT` before `setup-benchmark.sh`/`run-benchmark.sh`; the
resolved value is persisted into the generated case. A case created with
`BENCHMARK_AGENT=opencode` runs its worker/reviewers/analyzer under the
opencode driver later, and its copied telemetry/session-id scripts resolve the
driver via `REPO_ROOT` from `benchmark-env.sh` (with a `dirname` fallback).

Telemetry is always honest: only the active agent's documented store is read,
and any missing/invalid identity yields `telemetry_status=unavailable:...`
rather than a fabricated token count.
