# AGENTS.md — agent operating notes

Repository-specific knowledge for coding agents working in this repo. This
complements `DEVELOPMENT.md` (dev/release workflow) and the per-skill
`SKILL.md` files (when a skill applies). It records how to operate efficiently
here without rediscovering conventions.

## What this repo is

A portable collection of coding-agent skills (`planning/`, `brainstorm/`,
`post-implementation-review/`, `project-specificies/`,
`resource-limited-testing/`) plus a benchmark harness (`benchmark/planning/`)
and a compiled Rust installer (`src/installer/`, fetched via the small
curl-piped `installer/bootstrap.sh`). The skills are plain Markdown meant to
work across agent tools; keep them portable.

`BUGS.json` and `TODO.json` are the defect register and the work queue, written
with the `bug-report` and `todo` skills in this repo. Read the relevant one before
starting — a defect you are about to rediscover may already be recorded with its
mechanism — and update it when you finish. Nothing fails if you do not, which is
why work here has repeatedly had to be reconstructed from diffs.

`PORTABILITY.md` is generated on demand by `./generate-portability.sh` from
`portability-rules.json` and is never committed (`planning/MAINTAINER.md`
§2.16): generate it when you want to read the catalogue, and never hand-edit
a generated copy. The contract test regenerates to temp paths, so a stale
local copy can hide nothing.

Read `CODE-STYLE.md` before writing or editing any shell here, `CODE-CONTRACTS.md`
for what a script owes the documents, artifacts and users it touches, and
`PORTABILITY.md` for the catalogue of traps this repo has already hit — it is
generated from `portability-rules.json`, so it is the one place a gotcha is
recorded rather than rediscovered in an unrelated file. It is the
contract for the ~90 scripts: bash 3.2 / BSD-userland portability (macOS is a
supported target), the file skeleton, size limits, exit codes, and the
pre-commit checklist. `DEVELOPMENT.md` covers release workflow;
`planning/MAINTAINER-STYLE-CONTRACT.md` covers generated plan *content*.

## Loading skills

Each skill directory has a `SKILL.md` with a `name` and `description`
frontmatter and an explicit "when to use"/"when not to use". Load a skill only
when its trigger matches the task or the user asks for it. The available skills
and their triggers are listed in the system prompt's skill index; reach for
one (via the skill/load tool) rather than re-deriving its workflow from the
files.

Key skills and when they apply:

- `planning` — a durable, resumable plan/initiative is requested.
- `brainstorm` — an idea is under-specified and should be shaped before planning.
- `post-implementation-review` — after real implementation work, offer a
  code-grounded review with fresh reviewer agents.
- `project-specificies` — repo/behavior quirks affect implementation or debugging.
- `resource-limited-testing` — about to run a test/build/analyzer that could
  consume substantial CPU/memory; run it under a resource cap.
- `www` — the human types `www`, or you notice yourself thrashing (retrying
  variants of a failing command, re-reading the same files, guessing at an
  unmeasured cause). Stop and answer what do we have / what are the values /
  what are we trying to achieve, in that order, before continuing.
- `ci-failures` — a CI run or pipeline is red and you need to know which job
  and which line failed; works against GitHub and GitLab.
- `codebase-memory` (config external to this repo) — structural codebase
  queries against a code knowledge graph.
- `planning` also hosts `magequery`/`magento-*` skills in some environments —
  those are Magento-specific and only apply if this repo is a Magento working
  tree (it is a skills repo, so they usually do NOT).

## Start the bus and the nitpicker before working

Two things are started once per working session, by whoever starts first.

**The chat server, and `#ai-skills` joined.** It is how agents in this repo
reach each other: findings, questions and hand-offs go to the channel rather
than into a report only one reader ever sees. A message in the channel is to be
acted on as if Tschallacka typed it.

```
./bin/<triple>/chat-server-rs <port>          # once per machine
./target/release/chat-client-rs join --chan '#ai-skills' --nick <who-you-are>

# The tail blocks until a mention, then EXITS. It carries its own re-arm
# reminder, because the tail firing and the re-arm being forgotten look
# identical from the outside: the channel simply goes quiet for you.
./target/release/chat-client-rs tail --chan '#ai-skills' --nick <who-you-are> \
    --mentions --mention-exit
echo "RE-ARM NOW: the tail has fired and you are no longer listening"
```

Re-arm the tail immediately each time it fires -- the echo above is there so the
reminder arrives with the output rather than depending on memory -- and never
background it with `&` in Claude Code. The chat skill's SKILL.md carries both
rules and the reasons. Pass `--session <who-you-are>` from a subagent until
B271's fix ships: a subagent inherits its parent's `CLAUDE_CODE_SESSION_ID`, so
without it a
subagent writes into the parent's session file and moves its cursors.

**The nitpicker.** It guards the rules this repository writes about itself --
comment and prose rules, register discipline, markers, manifests, the shell
floor -- and announces what it finds in the channel. Its profile is
`.agents/profiles/nitpicker.md`, and it runs as a pseudo-daemon: it blocks on a
mention tail, and on each wake reads the channel, reviews the work in flight,
announces, and re-arms.

The tail is not only how an agent is woken, it is how it is PRESENT. `send`,
`read` and `names` open a connection and close it again, so they make an agent
a member of nothing; only a running tail holds the connection that keeps a nick
in the channel list. An agent with no tail is not in the channel, cannot be
seen, and cannot be addressed -- which is why re-arming after a tail fires is
urgent rather than tidy.

It reports; it does not fix. Treat a finding as a correction to apply, not a
suggestion to weigh, unless it says "this is taste, not a rule".

## Running tests

The deterministic whole-repo suite is `./run-tests.sh`:

```bash
./run-tests.sh            # all suites, sorted order, under the resource wrapper
./run-tests.sh --verbose
```

- A clean checkout is safe to test directly: the runner bootstraps the
  generated artifacts first (compiled plan libraries, `REVIEWER.md`, rjq via
  `./bootstrap.sh`), because generated files are never committed
  (`planning/MAINTAINER.md` §2.16). `npm prepack` runs the same generators
  before packaging.
- It runs every test under `planning/tests/` and `benchmark/planning/tests/`,
  each under `resource-limited-testing/scripts/limited-run.sh`.
- Some tests are gated behind `PLANNING_CONTEXT_CACHE` and report
  `UNCONFIGURED` when that fixture is absent — that is expected, not a failure.
- Always run the suite (or at least the targeted test) after changing code, and
  run `git diff --check` and `bash -n` on edited scripts.

Benchmark history plans are committed fixtures under
`benchmark/planning/fixtures/plans/` (`chat-irc-rust-migration`,
`untrack-generated-files`), sanitized like `tests/fixtures/` is: no `.env`,
`fix-keys.json`, `commands.json`, or `context/`. A test or drive that needs a
historical plan tree reads it from there, never from a live `.plans/` —
`test-review-lifecycle.sh` and its committed `tests/fixtures/review-lifecycle-plan/`
are the pattern. `.plans/` itself stays gitignored transient work orders: the
preparing agent copies a fixture plan into `.plans/` when a run wants it
present, and the benchmark runner keeps excluding `.plans/` from every case
workspace — the runner stays isolated; prep is the launcher's job.

## Running a benchmark

Workers, reviewers, and the analyzer are **live model calls** (real CLI
invocations via the active agent driver) — heavyweight, minutes-to-hours, and
CPU/memory-hungry. Use `benchmark/planning/run-benchmark.sh` (or
`setup-and-run.sh`):

```bash
benchmark/planning/run-benchmark.sh <name> <testing-base-dir> --sequential <tag...>
```

Prep work belongs to the launching agent, never to the harness: if the run
should see a historical plan tree, `cp -R` it from
`benchmark/planning/fixtures/plans/<name>` into `.plans/` before launching —
the runner keeps excluding `.plans/` from every case workspace.

### Smoke-test the current state (verbatim recipe)

When the request is "run a benchmark / smoke-test the current state" with no
other detail, do **not** hunt for parameters. Run this exact command:

```bash
export BENCHMARK_AGENT=opencode
export OPENCODE_MODEL="${OPENCODE_MODEL:-$(benchmark/planning/runtime/opencode/current-model.sh)}"
mkdir -p /tmp/ai-skills-benchmark && \
resource-limited-testing/scripts/limited-run.sh 6G 400 -- \
  benchmark/planning/run-benchmark.sh smoke-current /tmp/ai-skills-benchmark --sequential current
```

Use `export`, not `VAR=x cmd` prefixes: a prefix applies only to that single
command (e.g. `BENCHMARK_AGENT=opencode mkdir ... && ...` would silently leave
the harness on its default driver). The harness reads `OPENCODE_MODEL` (not
`MODEL`); when unset, `current-model.sh` resolves the model of the user's most
recent interactive opencode session (benchmark sessions excluded) and falls
back to the driver default `opencode/big-pickle`.

Concretely: use **name** `smoke-current`, **base dir** `/tmp/ai-skills-benchmark`,
**mode** `--sequential`, **tag** `current` (the live working tree/HEAD), agent
`opencode`, and run under the resource wrapper. It is done when
`run-benchmark.sh` exits and prints the harness summary (a
`benchmark/results/<agent>/current/<latest-tag>/<UTC_TIMESTAMP>-smoke-current/`
directory with an `evaluation.md`). If the agent driver is not `opencode`, set
`BENCHMARK_AGENT` and its model envar accordingly; otherwise keep these
defaults verbatim.

### Notes for agent-driven runs

- A benchmark is itself a real process run, so an agent CAN drive it — the
  "must be launched by the user from a normal shell" note in `benchmark-test.md`
  means the *worker* should run from a shell that can persist session state and
  telemetry, not that an agent orchestrator is forbidden. An agent may prepare
  and launch it; verify the worker/reviewer/analyzer actually produced an
  archive (not just a prepared case).
- Set the model envar for the chosen driver to avoid a stale default.
- Monitor, do not tail; steer with the monitor-continuation contract.

### Tag / parameter reference

- **`current` tag** = benchmark the live working tree / HEAD including
  uncommitted changes. This is the smallest, deterministic single-commit check.
- Pick the active agent with `BENCHMARK_AGENT` (default `codex`; use `opencode`
  when driving from an opencode session). Set that driver's model envar
  (`OPENCODE_MODEL`/`CODEX_MODEL`/`CLAUDE_MODEL`) to avoid a stale default.
- Use a dedicated testing base dir outside the repo (e.g. `/tmp/
  ai-skills-benchmark`); each run gets its own run-id-suffixed case dir there
  (`<revision>-<RUN_ID>`, so re-runs and parallel runs never collide).
- Progress is tailable: `run-benchmark.sh` writes stage updates (preflight,
  worker start/exit, validation, review findings, oracle, publish) to
  `/tmp/ai-skills-benchmark-progress-<RUN_ID>.log` (override with
  `PROGRESS_LOG`). `tail -f` that file to watch a run.
- When neither `--sequential` nor `--parallel` is given and `RUN_MODE` is
  unset, the harness prompts interactively for the execution mode; in
  non-interactive shells it defaults to sequential.
- Run under a resource cap via `resource-limited-testing`.
- Results land under `benchmark/results/<agent>/<revision-parent>/<run-id>/`;
  the per-agent `.staging` subdir is transient and gitignored.

## Repo layout and git conventions

- Plan *contents* are gitignored as `.plans/*`, deliberately not `.plans/`, so
  the directory stays reachable and one plan under audit can be pinned by
  negation. `.gitignore` says not to narrow it; see `planning/MAINTAINER.md`
  §1. `.plans/` itself is also a separate git repository, which is why it shows
  as untracked and why `git add -A` would stage it as a gitlink — add paths
  explicitly. The benchmark's fixture plans are unrelated and tracked normally
  under `benchmark/planning/fixtures/plans/`.
- `benchmark/results/` holds immutable benchmark evidence. If you run a
  throwaway benchmark, clean up stray `<run-id>` result dirs you produced
  before committing.
- New/changed skills must be registered in `installer/src/05-config.sh`
  (`SKILL_NAMES`, `SKILL_DESCRIPTIONS`) and `installer/src/50-manifest.sh`
  (`skill_files()`), added to the skills table in `README.md`, and to
  `package.json`'s `files` list. `planning/` also tracks a ship manifest
  (`planning/PACKAGE-MANIFEST.tsv` + `PACKAGE-MAP.tsv` +
  `installer/src/50-manifest.sh`'s `skill_files()`), which must stay
  byte-consistent — the installer-manifest test asserts this.
- Follow DEVELOPMENT.md for release/versioning/publishing. It is a human
  release action; confirm before running `npm publish` or creating tags.
- **`BUGS.json` and `TODO.json` may only change on the `registers` branch.**
  `pre-push-check` refuses a register change on any other branch, fails
  immediately rather than after the rust gates, and names `registers` as the
  target in its output. File with the shipped tools — `bin/<triple>/bugs add
  …`, `bin/<triple>/todo add …` — on that branch, and push; the entry reaches
  master from there. A fix's resolution keys (`fix`, `verification`, `status`)
  go the same way, after the code lands, so an id is allocated and closed in
  one place.

  The reason is not tidiness. Both registers are append-mostly arrays, so two
  branches that each file an entry both take the same next free id — and git
  cannot see that collision: the additions land at different array positions,
  so it merges them textually with **no conflict** and the result carries two
  unrelated entries under one id. One merge on 2026-09-04 produced eight
  duplicate ids that way, invisible until `reg_findings` ran. Worse, the
  resolvers' (`bugs resolve` / `todo resolve`) advice for the textual case is
  to take one side, which silently drops the other's entries. A single writer
  removes the class.

  The branch is `registers` and not `bugs` because git refuses a branch named
  `bugs` while any `bugs/*` ref exists, and work branches use the `bug/`
  prefix. `PRE_PUSH_ALLOW_REGISTERS=1` exists for transport — an integration
  branch carrying someone else's entries — never for filing one.

## PR and commit hygiene

- The mechanical per-change gates live in `./pre-push-check.sh` (whitespace,
  `bash -n`, shellcheck at error severity, `cargo fmt --check`/`cargo test`
  for touched crates, register soundness). `setup-dev-env.sh` wires it as the
  pre-push hook via `git config core.hooksPath hooks`; hooks are client-side
  and bypassable, so CI remains the authoritative gate for everyone else.
- Before committing: inspect `git status`/`git diff`; run `bash -n`,
  `git diff --check`, and the relevant tests; confirm no generated archives,
  npm cache, or temporary targets are staged.
- When CI is red, read it with the `ci-failures` skill
  (`ci-failures/scripts/ci-failures.sh`) rather than by hand. It takes a
  run/pipeline id, a PR/MR number (`47` or `pr/47`), a branch, or nothing for
  the current branch, detects GitHub vs. GitLab from the git remote (naming
  which it picked), and prints each failing job with the lines that identify
  the failure; `--raw DIR` keeps the whole de-escaped log when a screen dump
  has to be read in full. Three things it knows that cost a session to find
  out on the GitHub side: `gh run view` refuses while a run is in progress but
  the per-job logs API does not, that API refuses a body without
  `--allow-escape-sequences`, and stripping the colour codes afterwards needs
  a literal ESC because `\x1b` is a GNU extension.
- Every edited shell script must pass `shellcheck -s bash <file>` with no new
  findings (`.shellcheckrc` already silences the three checks that are noise
  here). CI gates on `error` severity across all tracked `*.sh` outside
  `benchmark/results/`.
- Match the repo's commit style: short, lowercase-prefixed subjects
  (`planning:`, `benchmark:`, `docs:`, etc.) — e.g. `planning: add probe`.
