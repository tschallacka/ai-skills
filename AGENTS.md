# AGENTS.md — agent operating notes

Repository-specific knowledge for coding agents working in this repo. This
complements `DEVELOPMENT.md` (dev/release workflow) and the per-skill
`SKILL.md` files (when a skill applies). It records how to operate efficiently
here without rediscovering conventions.

## What this repo is

A portable collection of coding-agent skills (`planning/`, `brainstorm/`,
`post-implementation-review/`, `project-specificies/`,
`resource-limited-testing/`) plus a benchmark harness (`benchmark/planning/`)
and a shell installer (`install.sh`). The skills are plain Markdown meant to
work across agent tools; keep them portable.

`BUGS.json` and `TODO.json` are the defect register and the work queue, written
with the `bug-report` and `todo` skills in this repo. Read the relevant one before
starting — a defect you are about to rediscover may already be recorded with its
mechanism — and update it when you finish. Nothing fails if you do not, which is
why work here has repeatedly had to be reconstructed from diffs.

`PORTABILITY.md` is generated and carries a `<!-- generated: … -->` stamp; if it
looks stale or a test says so, run `./generate-portability.sh` — never hand-edit
it, and never hand-resolve a conflict in it. Concurrent edits invalidate it
routinely; regenerating is the fix.

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
- `codebase-memory` (config external to this repo) — structural codebase
  queries against a code knowledge graph.
- `planning` also hosts `magequery`/`magento-*` skills in some environments —
  those are Magento-specific and only apply if this repo is a Magento working
  tree (it is a skills repo, so they usually do NOT).

## Running tests

The deterministic whole-repo suite is `./run-tests.sh`:

```bash
./run-tests.sh            # all suites, sorted order, under the resource wrapper
./run-tests.sh --verbose
```

- It runs every test under `planning/tests/` and `benchmark/planning/tests/`,
  each under `resource-limited-testing/scripts/limited-run.sh`.
- Some tests are gated behind `PLANNING_CONTEXT_CACHE` and report
  `UNCONFIGURED` when that fixture is absent — that is expected, not a failure.
- Always run the suite (or at least the targeted test) after changing code, and
  run `git diff --check` and `bash -n` on edited scripts.

Tests that reference a specific `.plans/<plan>/` directory as fixture data
(e.g. `benchmark/planning/tests/test-review-lifecycle.sh` → `.plans/
reviewer-oracle-evidence-hardening/`) depend on gitignored transient plans. If
those are missing, restore them from git history (`git log --all -- .plans/`,
`git archive`) or make the test self-contained — do not silently delete the
.gitignore rule.

## Running a benchmark

Workers, reviewers, and the analyzer are **live model calls** (real CLI
invocations via the active agent driver) — heavyweight, minutes-to-hours, and
CPU/memory-hungry. Use `benchmark/planning/run-benchmark.sh` (or
`setup-and-run.sh`):

```bash
benchmark/planning/run-benchmark.sh <name> <testing-base-dir> --sequential <tag...>
```

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

- `.plans/` and `MEMORY.md` are gitignored (transient plans + agent handoff
  note). Never commit them. The benchmark fixture plans under `.plans/` are a
  known gitignored test-data dependency — restore from git history, not added
  to the ignore-allowlist.
- `benchmark/results/` holds immutable benchmark evidence. If you run a
  throwaway benchmark, clean up stray `<run-id>` result dirs you produced
  before committing.
- New/changed skills must be registered in `install.sh` (`SKILL_NAMES`, the
  shop menu, `select_skills`, and copy logic), added to the skills table in
  `README.md`, and to `package.json`'s `files` list. `planning/` also tracks a
  ship manifest (`planning/PACKAGE-MANIFEST.txt` + `PACKAGE-MAP.tsv` +
  `install.sh skill_files()`), which must stay byte-consistent — the
  installer-manifest test asserts this.
- Follow DEVELOPMENT.md for release/versioning/publishing. It is a human
  release action; confirm before running `npm publish` or creating tags.

## PR and commit hygiene

- Before committing: inspect `git status`/`git diff`; run `bash -n`,
  `git diff --check`, and the relevant tests; confirm no generated archives,
  npm cache, or temporary targets are staged.
- Every edited shell script must pass `shellcheck -s bash <file>` with no new
  findings (`.shellcheckrc` already silences the three checks that are noise
  here). CI gates on `error` severity across all tracked `*.sh` outside
  `benchmark/results/`.
- Match the repo's commit style: short, lowercase-prefixed subjects
  (`planning:`, `benchmark:`, `docs:`, etc.) — e.g. `planning: add probe`.
