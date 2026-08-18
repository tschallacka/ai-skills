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
| `agent_argv_worker <workspace> <capsule> <prompt>` | function | fills `AGENT_ARGV` and `AGENT_CWD` for a worker run: `AGENT_CWD=<workspace>`, capsule/workspace made readable, `<prompt>` content as the message. |
| `agent_argv_reviewer <workspace> <capsule> <prompt> [binary]` | function | fills `AGENT_ARGV`/`AGENT_CWD` for a Reviewer A/B run. Optional `$4` is the `REVIEWER_COMMAND` test seam: it replaces the driver binary (argv[0]) rather than appending codex flags. For a non-codex driver the seam is documented codex-only; when set it must not leak invalid flags. |
| `agent_argv_analyzer <workspace> <capsule> <prompt>` | function | fills `AGENT_ARGV`/`AGENT_CWD` for the batch analyzer run. |
| `AGENT_CWD` | variable (set by every `agent_argv_*`) | the directory the agent must run in — always the workspace. `launch_agent` cds there before exec, because not every CLI has a cwd flag (codex `-C`, opencode `--dir`, claude none) and `env -C` is GNU-only. |
| `agent_session_id <agent-output.jsonl>` | function | prints the session id extracted from the agent's output stream, or empty for an honest degrade. Parse line by line and return 0 on a missing/unparsable stream: the file is JSONL with the agent's stderr merged in, and the harness turns empty output into `SESSION_ID=unavailable` (tainted) rather than aborting the case. |
| `agent_telemetry <session-id>` | function | prints one `KEY=VALUE` per line, each key at column 0, first match per key wins: `thread_id`, `usage_records`, `total_usage_tokens`, `telemetry_status` (plus optional `telemetry_db`/`telemetry_source`). Never fabricate token numbers; when no documented store exists for the agent print `telemetry_status=unavailable:...`. |

The resolved model for the active driver is `"${!agent_model_env:-agent_default_model}"`;
`agent_resolve_model` (from `lib-agent.sh`) prints it. `agent_argv_*` functions
must call it so each role honours `CODEX_MODEL`/`OPENCODE_MODEL`/`CLAUDE_MODEL`.
A driver is **sourced**, never executed, so it must never `exit`: guard arity
with `[ "$#" -eq 3 ] || return 64` and return a sysexits code, letting the
harness decide what an argv failure means. Never hardcode a model in the argv
either — that silently stops the role honouring the harness override.

## Active-agent resolution

`agent-env.sh` `resolve_active_agent <runtime_dir>` exports `AGENT_DRIVER`:

1. `BENCHMARK_AGENT` set and a driver exists under `runtime/<agent>/` → selected.
2. `BENCHMARK_AGENT` set to an unknown agent → fail closed (exit 64). Because
   `lib-agent.sh` is *sourced*, it propagates that status with `exit "$?"`, so a
   typo in `BENCHMARK_AGENT` refuses the run instead of producing an empty,
   apparently successful batch.
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
  (set by a driver argv function first) from `AGENT_CWD` in the background, then
  records `AGENT_PID` and (for `setsid` mode) `AGENT_PGID`.
  - `mode=setsid` wraps with `setsid --wait` and the given `timeout` (e.g.
    `45m`); this is used by workers and reviewers. `setsid` **forks** rather
    than execs when it already leads a process group, so without `--wait` `$!`
    can be a parent that exits 0 while the agent still runs — `wait_agent`
    would then record `AGENT_EXIT=0` for an unfinished agent and the harness
    would read `worker.jsonl` before it was written.
  - `setsid` and `timeout` are Linux/util-linux + GNU tools. Both are guarded:
    a host without `setsid` gets exit 69 with a message, and `gtimeout` (GNU
    coreutils on macOS) is used when `timeout` is absent.
  - `AGENT_PGID` is polled rather than read once, because a not-yet-scheduled
    agent has no observable process group and an empty `AGENT_PGID` silently
    disables the process-group kill and taints the run's process audit.
  - `mode=background` is a plain background run with **no setsid and no
    timeout**; it preserves the pre-refactor analyzer semantics exactly. That
    means the batch analyzer — which gates the batch exit code — is unbounded
    by construction; see the risk note below.
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
   `--dir` + `-f` attachments enumerated from the capsule / `--auto`). Do not
   fake a normalized flag layer; record the mechanism in the driver. Never
   hardcode capsule-relative paths: the three capsules have different layouts
   (worker nests `planning/`, reviewer is flat with the reviewed plan under
   `plan/`, analyzer holds `benchmark-test.md` + `harness-summary.tsv` +
   `results/`), so enumerate what the role was handed.
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

To benchmark only the current working tree / HEAD (including uncommitted
changes), use the reserved `current` revision tag rather than a version tag:

```bash
BENCHMARK_AGENT=opencode benchmark/planning/setup-benchmark.sh current <dir> <name>
```

`current` archives the live repo (excluding `.git`, `.plans`,
`benchmark/results`) instead of running `git archive` on a tag; see
`benchmark/planning/README.md` and `benchmark-test.md`.


Telemetry is always honest: only the active agent's documented store is read,
and any missing/invalid identity yields `telemetry_status=unavailable:...`
rather than a fabricated token count.

## Known risk: the analyzer has no timeout

`run-benchmark.sh` launches the batch analyzer with `launch_agent background ""`,
which by the definition above applies neither `setsid` nor a timeout — and the
analyzer's exit code is what gates the whole batch's exit code. A hung analyzer
therefore hangs the batch indefinitely, after every worker and reviewer has
already been paid for. This is deliberate (it preserves the pre-refactor
analyzer semantics) and asserted by `tests/test-safeguards.sh`, so changing it
is a maintainer decision, not a cleanup.

## Known risk: the `REVIEWER_COMMAND` seam can spend real tokens

`REVIEWER_COMMAND` substitutes argv[0] only, so an override still receives the
active driver's flags. `setup-benchmark.sh` guards the non-codex case by
*ignoring the seam and running the real driver*, which means a test that sets
`REVIEWER_COMMAND` under `BENCHMARK_AGENT=opencode` (or `claude`) silently
launches a live reviewer and spends real model budget instead of failing. A
non-codex seam should refuse the run rather than fall through to the real agent;
that is a maintainer decision because it changes harness behaviour under test.
