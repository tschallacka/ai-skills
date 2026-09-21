<!-- MODE: DEV -->
# Repository Handbook — behavior rules & change checklist

**Audience: agents and maintainers.** This is the repo-wide, agent-facing
contract: the rules that govern how every skill and helper in this repository
is built and changed. The planning skill's own architecture and format details
live in `planning/MAINTAINER.md` and `planning/MAINTAINER-STYLE-CONTRACT.md`;
this file holds what applies to the repository as a whole, and is the one home
for those rules: `planning/MAINTAINER.md` keeps only the planning skill's own
material and points here for the rest.

**This file does not ship.** What ships to end users is pure binaries and the
skills that use them. The maintainer documents, this file and everything it
links to (`.agents/knowledge/`, `docs/`, `CODE-STYLE.md` and the rest), are
`MODE: DEV` or otherwise left out of the release and the npm package; they exist
in a full git checkout of the repository, and only there.

## Start here

The order a newcomer needs things in. Each step names what to run, the gotcha
that costs the most time, and where the full story lives; nothing here is
copied from the owning doc. Sections 1 to 3 below are the rules, the gate and
the CI map.

1. **Orient.** `AGENTS.md` is how agents operate here (loading skills, the chat
   bus, running tests, running a benchmark, which makes live model calls and
   whose `benchmark/results/` is immutable evidence, repo layout, PR and commit
   hygiene). `CONTRIBUTING.md` is
   the dev environment, portability gotchas, linting and the pre-PR list.
   `MEMORY.md` holds the diagnostic lessons that are neither rules nor defects.
   `DEVELOPMENT.md` is the installer, npm package and versioning workflow, and
   `RELEASE.md` the release protocol. `.agents/MAINTAINER-STYLE-CONTRACT.md` is
   the repo-wide contract for how documents and generated formats are written and
   how evidence is treated (benchmark reports and archives are immutable).
2. **Build the tree.** `./setup-dev-env.sh` (1.9). **Nix is mandatory and every
   developer flow goes through it**, so exit 69 means no nix; `nix develop` is
   the dev shell, with `bash32`,
   `bash32-run-tests`, `bash32-run`, `shellcheck`, `mmdc` and the pinned cargo.
   On a machine that has skills installed, export
   `AI_SKILLS_BIN_ROOT=$PWD/bin/<triple>` before `./run-tests.sh` or
   `./pre-push-check.sh`: the binary lookup does not fall through, and the dev
   shell's PATH does not cure it (1.9).
3. **Write to the contracts.** `CODE-STYLE.md` is how a shell file is written
   (the bash 3.2 floor, size limits, exit codes). Its section 11, on comments, is
   the rule people break: a function's comment is three lines at most, and a
   comment carries no measurements, no cross-file pointers ("see also X") and no
   development history. `CODE-CONTRACTS.md` is how
   scripts must behave; its contract 10a is why every file carries `MODE` and
   `PACKAGE` markers. `PORTABILITY.md` is a generated catalogue of banned
   constructs, untracked: never edit it, `setup-dev-env.sh` and the gate
   regenerate it.
4. **Run the tests.** `./run-tests.sh [--verbose] [--select-file FILE] [--shard
   I/N]`, or `--list-only`. It is a shim over the compiled `run-tests` and exits
   69 without it. An unknown argument exits 64, so `./run-tests.sh <word>` is
   not a name filter: use `--select-file`. One run at a time machine-wide
   (`/tmp/ai-skills-run-tests.lock`, exit 75, `AI_SKILLS_ALLOW_CONCURRENT=1`
   bypasses it), each test bounded by `AI_SKILLS_TEST_TIMEOUT` (default 600 s),
   and run under the resource-limit wrapper unless `GITHUB_ACTIONS` is set or the
   host is Windows (`AI_SKILLS_RESOURCE_LIMIT=0` or `1` overrides). A failing
   test's evidence is in 1.12 and `docs/DEBUGGING-TESTS.md`; scratch space and
   running one test or one crate are 1.12a.
5. **Check the floor.** `bash32-run-tests` runs the suite on bash 3.2.
   `./verify-both-shells.sh` runs it under both shells in a detached worktree:
   one at a time, and never edit the `.sh` while a run is in flight (`BUGS.json`
   B16, B17; `DEVELOPMENT.md`, "Verifying on both shells"). A BSD-only failure
   shows only on the macOS CI legs.
6. **Check what a change touches.** `blast-radius.sh` reports stale generated
   artifacts, a new file under `planning/` with no manifest row, commits that
   moved these files since the base, and the couplings in `coupling.tsv` that a
   human must honour.
7. **Commit and push.** The commit-msg hook and the pre-push gate are 1.17;
   register entries are filed only on the `registers` branch (1.14).
8. **Read CI.** Section 3 maps every workflow and job to what it proves.

**Where things are.** Every skill has its own directory at the root; the rest:

| directory | what it is | read |
|---|---|---|
| `src/` | the Rust workspace, one crate per directory, each with its own `tests/` | 2b |
| `tests/` | repo-wide shell tests (`test-*.sh`) and `tests/rust-support/`, shared Rust test helpers; the planning skill's shell tests are in `planning/tests/` | 1.12a |
| `installer/` | release and bootstrap tooling (`build-release.sh`, `bootstrap.sh`, and `installer/src/*.sh`, which holds `skill_files()`) | 2a, `RELEASE.md` |
| `benchmark/` | the planning benchmark harness; `benchmark/results/` is immutable evidence | `AGENTS.md` |
| `testing-stories/` | hands a skill to a fresh agent in Docker; every real run is a billed agent session and nothing here runs automatically | `testing-stories/README.md` |
| `hooks/` | the git hooks `setup-dev-env.sh` wires (`commit-msg`, `pre-push`) | 1.17 |
| `docs/` | `DEBUGGING-TESTS.md` and the interactive-shell `TESTING-PROTOCOL.md` | 1.12 |
| `agent-identity-plugin/`, `editor-gate-plugin/`, `tui-hint-plugin/` | agent-harness plugins built on hooks, each with a `README.md` and `tests/` | their `README.md` |
| `.brainstorm/` | a working note (`actionplan.md`, a reviewer-optimisation plan), not shipped | — |
| `bin/`, `target/` | generated output: built binaries and cargo's build directory | 1.9, 1.10 |

## 1. Behavior rules

### 1.1 No backwards compatibility
- A changed command/format is a **clean break**. No aliases, legacy modes, or
  inferred defaults. Old forms fail loudly.
- Coordinated migration: update producer, parser/validator, fixtures, tests,
  manifest/map, `installer/src/50-manifest.sh`'s `skill_files()`, and the hash
  test in the **same** change.

### 1.2 Small, scoped, single-source docs
- A skill's `SKILL.md` stays a lean index and shared contract. Never let it
  regrow into a monolithic document.
- **This rule has a measured limit behind it, not just a preference.** Claude
  Code returns a *prefix* of a file at roughly 25,000 tokens and says nothing:
  no notice in the tool result, no notice anywhere. A document past that is
  partly read and reads as fully read. opencode caps an attachment at 50 KB but
  does say so; codex did not truncate. Measured 2026-09-03 against Claude Code
  2.1.259, opencode 1.18.27 and codex 0.153.0. The working notes are kept in
  the repository at `.agents/knowledge/agent-read-limits.md`, which is
  maintainer-only and not part of a release.
- Agents read only the doc for the task they are doing. Phase or scope scoping
  prevents "future knowledge" leaking into a wrong context.
- Shared facts live in exactly one place. Docs **reference, never duplicate** —
  a second copy is a drift hazard, not a convenience.

### 1.2a Measured facts go in `.agents/knowledge/`
- A limit, a silent behaviour or a refuted belief that cost time to establish
  belongs in `.agents/knowledge/`, with its numbers and how it was measured.
  It is not a changelog: entries answer one question each and must be
  re-checkable from what they contain. That directory is maintainer-only
  (`MODE: DEV`), so it exists in the repository and not in a release.
- **Before diagnosing a platform-specific or tool-behaviour failure, scan the
  index at `.agents/knowledge/README.md`.** It lists what is already measured:
  what GitHub's runners are like (arch, speed, `$TMPDIR`), why a unix socket
  bind that works on Linux fails on macOS, what each agent harness reports
  about identity and reads of large files, why `grep` warns "stray \ before -",
  why tests that share a port fail only while another run is active, and why
  correct code fails on Windows.
- Queued work is `TODO.json` and defects are `BUGS.json`. Knowledge is neither
  — it is what the next agent would otherwise rediscover.

### 1.3 Proactive, reconciling tools
- When a mutation happens, the tool reconciles every reference it knows about
  automatically — coverage rows, cross-links, indices, trackers. Do not make an
  agent issue a follow-up call the tool knew it needed.
- Keep helpers **small**; put shared logic in library files.

### 1.4 Deterministic command contracts
- Every subcommand has one fixed, documented positional signature. An explicit
  `document-id` where applicable. No positional overloading, no value-sniffing.
- Every mutating helper has `--help` (exit 0, concise) and **actionable
  errors**: state the problem and what the agent can do to resolve it.

### 1.5 Identity-gated capabilities
- Revealing capabilities are gated by caller role (`ROLE_ID`); `--list`
  (id/name only) is deliberately open. Default print mode only ever emits the
  requested role's own docs.
- Content reads FAIL CLOSED: an unset or unknown `ROLE_ID` is a hard refusal
  with a `FAIL-CLOSED identity` message — the worker is denied a persona and
  must be respawned.
- Shell gates are **advisory, not a security boundary**; the agent framework
  confines the process. Document that.

### 1.6 Roles are a canonical registry
- Canonical ids/names are assigned once. Names are meaningful only alongside
  the id; never repurpose a name.
- Each skill keeps its own role registry in a maintainer contract; agent-facing
  scope docs reference ids, never redefine them.

### 1.7 Review protocol invariants
- Reviewers fall into two classes: handoff-only (may verify but never approve)
  and sole-approval-authority. Adversarial review is done by a fresh secondary
  agent with no access to the planner's conclusions.
- Validate before creating progress trackers; a plan is not ready until the
  review is approved and validation passes.

### 1.8 One EXIT trap, process-wide
- A shared library installs a single cleanup on `EXIT INT TERM` at load and
  keeps one accumulating temp list. Never use `trap - EXIT` to "release" a
  per-call handler; that clears the process-wide slot (CODE-STYLE §8).

### 1.9 A working local tree needs the crates built
- Run `./setup-dev-env.sh` once after cloning. It builds the binaries that
  `./setup-dev-env.sh --list` prints, for this machine's target triple, into ONE
  `bin/<target triple>` at the repository root — the directory the plan helpers
  resolve as the shared binary home (`plan_bin_dir`). That list is a curated set
  of rows, not every crate under `src/`: the library crates and some binaries
  (for example `installer` and `generate-profile-content`) have no row.
- It also places copies where a skill looks for them: the planning commands go
  into `planning/scripts/<command>`, and the `bug-report`, `todo`,
  `interactive-shell` and `interactive-shell-mcp` binaries go into
  `<skill>/bin/<triple>/` as well as the shared bin.
- It generates what a fresh clone lacks and git ignores: the five
  `planning/scripts/plan-*-lib.sh` (via `build-plan-libs.sh`),
  `planning/REVIEWER.md` (via `generate-reviewer.sh`) and `PORTABILITY.md`. The
  libraries and `REVIEWER.md` are built **only when missing**: a present but
  stale copy is deliberately left alone, so after changing their sources rerun
  those two scripts yourself (1.13 is the trap this sets). `PORTABILITY.md` is
  regenerated on every run. It also runs `git config core.hooksPath hooks`,
  which is what wires the git hooks (1.17).
- **Nix is mandatory, and every developer flow goes through it** (the owner's
  rule). It is a development dependency only, and always has been: the flake is
  the dev environment, and nothing it provides is needed to *use* the skills or
  the installer. **Every development dependency goes in the flake** (the
  `default` shell in `flake.nix`): a doc never tells a contributor to install a
  tool by hand, and a step that needs a tool the shell lacks means adding it to
  the flake. `setup-dev-env.sh` re-enters `nix develop` itself, and exits 69 when
  nix is missing. The markers that skip that entry, `SETUP_DEV_ENV_IN_NIX` and
  `IN_NIX_SHELL`, exist for CI runners, which have a rustup toolchain and no nix,
  and for a shell you are already inside; they are not a way for a contributor to
  avoid nix.
- **Exit codes:** 64 bad usage; 69 nix is missing; 70 a crate failed to build. A run writes
  `.setup-dev-env.started` when it begins and `.setup-dev-env.finished`, with the
  same run token, only if everything built; `run-tests.sh` and the test library
  refuse a tree where `.started` has no matching `.finished` (`BUGS.json` B156),
  so a run killed halfway cannot pass for a finished one. `./setup-dev-env.sh
  --check` reports what is present without building. `./bootstrap.sh` builds the
  one tool the suite needs (`rjq`) for the runner's own bootstrap and for
  `npm prepack`, which run where the dev shell is not available; it is not an
  alternative to nix for a contributor.
- Only the host triple is built locally; cross-building the other targets is
  what a release (`installer/build-release.sh`) and CI do. **A tree with no
  compiled binaries cannot run the suite:** `run-tests.sh` is a shim over the
  compiled `run-tests` and exits 69 without it. What a local run cannot show is
  the other platforms' compiled paths (macOS, Windows, aarch64): only the CI
  legs in section 3 exercise those.
- **A machine's own `rjq` wins over the bundled one.** The plan helpers put
  `plan_bin_dir` on PATH themselves when they load, but only if no `rjq` is
  already on PATH (`planning/scripts/lib/document/99-facade.sh`), so an
  operator's pinned `rjq`, or a test's injected stub, is never overridden. For
  an interactive shell `setup-dev-env.sh` prints the `export PATH=` line for
  this host.
- **The Rust toolchain is pinned by literal in several places.** `rust-toolchain.toml`
  says `1.98`, and so do the `dtolnay/rust-toolchain@1.98` steps in `ci.yml`,
  `windows.yml`, `render-artifacts.yml` and `release-installer.yml`; the `test`
  job also checks `rustc --version | grep "1.98"`. `flake.nix` does not repeat
  it: it uses `rust-bin.stable.latest` from the locked overlay, so `nix flake
  update` can move a local shell past what CI runs, although
  `rust-toolchain.toml`'s own comment says the flake supplies "the same stable
  compiler". A bump edits every one of those places, including that `grep`.
- **An installed copy hides the tree you just built, and the lookup does not
  fall through.** `plan_bin_dir` returns the first of: `$AI_SKILLS_BIN_ROOT`
  (when it names a directory), then `${XDG_CONFIG_HOME:-~/.config}/tsch-ai-skills/bin`
  (when that exists), then the outermost `bin/<triple>` found walking up from
  the script, and looks for a binary by name in that one directory only. (The
  one exception, added for B365: `plan_exec_compiled_binary_if_present`, which
  every `scripts/*.sh` wrapper calls, then tries an executable of that name
  beside the wrapper, because an installed skill keeps its compiled commands in
  its own `scripts/` next to the wrappers. It comes last, so it never shadows
  the shared bin, and it is skipped whenever `AI_SKILLS_BIN_ROOT` is set: an
  override is authoritative, which is also what lets a test point it at an
  empty directory to simulate a missing binary in a checkout that stages a copy
  beside every wrapper.) A
  machine with skills installed has the second one, and it holds the shipped
  tools (`bugs`, `todo`, `rjq`, the chat and editor binaries), not the dev
  tools. So without `AI_SKILLS_BIN_ROOT`: `./run-tests.sh` exits 69 with "no
  compiled binary found ... run ./setup-dev-env.sh" on a fully built tree (the
  remedy is wrong, the lookup is the cause); `./pre-push-check.sh` silently runs
  its bash fallback, which prints less (1.12); and the tools the tests exercise
  are the installed ones, not yours. Export
  `AI_SKILLS_BIN_ROOT=$PWD/bin/<triple>` for every local `./run-tests.sh` and
  `./pre-push-check.sh`; every CI shard does the same after `setup-dev-env.sh`.

### 1.10 Generated files are CI's job, not the repo's
- Every binary and compiled output is built by a CI runner and delivered as a
  release artifact, or built locally by the generators below. The rule is that
  nothing machine-produced is committed, and it has reached its end state for
  every row of this table but the last: those files are untracked and
  gitignored (`git check-ignore -v <file>` names the rule). Adding a tracked generated file
  is a change to this rule, not an exception to it.

  | generated file | generator | on a fresh clone | what fails if it is missing, stale or tracked |
  |---|---|---|---|
  | `bin/<triple>/*` | `setup-dev-env.sh` (1.9); CI and a release build them per platform | absent | `run-tests.sh` exits 69; `tests/test-shipped-binaries.sh` fails if a bundled binary is tracked |
  | the five `planning/scripts/plan-{core,crypt,document,progress,table}-lib.sh` | `planning/scripts/build-plan-libs.sh`; `setup-dev-env.sh` runs it **only when one is missing** | absent | the planning helpers cannot source them; a stale copy that is present is worse (1.13) |
  | `planning/REVIEWER.md` | `planning/scripts/generate-reviewer.sh`; `setup-dev-env.sh` runs it only when it is missing | absent | `planning/tests/test-reviewer-projection.sh` (it pins `SKILL.md`'s SHA-256; edit `SKILL.md` and regenerate, never the projection) |
  | `PORTABILITY.md` | `generate-portability.sh`; regenerated by every `setup-dev-env.sh` run and every pre-push gate run (1.17) | absent | `planning/tests/test-portability-contract.sh` proves it is deterministic from `portability-rules.json` |
  | `.npmignore` | `installer/build-release.sh --npmignore` | **tracked**, the one exception; see 1.10a | nothing gates it: it goes stale until someone regenerates it |

- Why: a generated file in git is a blob that cannot be rebuilt on every
  maintainer box (there is no darwin or msvc link here), rots out of sight of the
  build that produces it, and every clone pays for it forever.
- A consumer that reads a generated file from the working tree — the installer,
  npm pack, test harnesses, `blast-radius.sh`'s freshness checks — has to build
  or fetch it first, because a clone does not carry it. The lint job is one of
  them: shellcheck resolves a `source=` directive only against files named on
  the same command line, so an untracked library drops out of `git ls-files`
  and every variable a sourcing script reads from it looks unassigned (SC2154).
  The `shellcheck` CI job therefore builds the libraries and appends them to its
  file list; a generated file that is linted, or sourced by something linted,
  belongs in that list.
- The installer is a compiled Rust binary (`src/installer/`), built at release
  time and shipped as a GitHub release asset, never committed; the old
  machine-assembled `install.sh` is gone (see git history).
  `installer/bootstrap.sh`, the small curl-piped entry point that downloads that
  asset and hands off to it, is committed because it is hand-written source, not
  generated; its own header explains why it carries the mascot art (copied from
  `installer/src/05-config.sh`) and the palette functions itself instead of
  sourcing them, and that `bootstrap.sh`, `ART` and
  `src/installer/src/ui/mascot.rs` are kept in sync by hand.

### 1.10a `.npmignore` and the npm size baseline

- **`.npmignore` is generated but tracked.** `installer/build-release.sh
  --npmignore > .npmignore` writes it from the `MODE` markers (everything not
  `MODE: PROD`), and its own header says to regenerate it after adding a file.
  **The generator reads the tracked files, so `git add` a new file first, or it
  is missing from the result.** Nothing in the pre-push gate or the suite checks
  it, so it is stale unless someone regenerates it; `RELEASE.md` re-checks it at
  release time. Check with `installer/build-release.sh --npmignore | diff -
  .npmignore` (no output means current).
- **The baseline pins packaged files' byte sizes.**
  `planning/tests/fixtures/overview/npm-package-baseline.tsv` has a row per
  pinned file, keyed as `package/<path>` with its size in bytes.
  **Editing any file that has a row (`README.md`, a `SKILL.md`, a shipped
  script) fails the gate's `npm package baseline drift` check until its row is
  refreshed.** To
  refresh: put the file's new `wc -c` size in its row, then confirm with
  `planning/tests/test-npm-package.sh`, which runs `npm pack` and takes minutes.
  A **new** packaged file has no row to disagree with, so the gate cannot catch
  a wrong selection; only that full test does. Files excluded from the package
  (`.agents/MAINTAINER.md`, `.agents/MAINTAINER-STYLE-CONTRACT.md`,
  `.agents/knowledge/**`, `planning/MAINTAINER.md`, tests) have no row.

### 1.11 CI runs: a push cancels the run it supersedes
- The workflow's concurrency group is keyed by ref
  (`ci-${{ github.workflow }}-${{ github.ref }}`) with `cancel-in-progress`, so
  **one pull request never cancels another's run**. On a `pull_request` event
  `github.ref` is `refs/pull/<N>/merge`, giving every PR its own lane. That
  keying is deliberate: a shared group once cancelled three open pull requests'
  runs through no fault of their own.
- What it does cancel is **the same branch superseding itself**. Push a second
  commit and the run for the first is killed. That is intended — finishing a run
  for a commit nobody will merge wastes a scarce macOS runner — and it is not a
  fault to fix.
- **So do not push while a run you need is still going.** A push destroys the
  previous run's result, and the macOS legs can wait many minutes for a runner
  before they even start. Batch the pushes, or hold them until the run
  completes. What that has cost, and the measured wait, are in
  `.agents/knowledge/github-ci-runners.md`.
- The distinction that matters: you lose the result **only when the earlier
  commit was the one being tested**. A superseded run of code you have already
  replaced is no loss at all.
- macOS runners are the scarce resource and set the critical path. A readiness
  or retry budget written for a Linux runner will be too tight there — the host
  is a shared, oversubscribed VPS that pauses for other tenants.

### 1.12 A failing test must be diagnosable from its output
- **`docs/DEBUGGING-TESTS.md` is the single source** for the evidence dump,
  breakpoints, `t_dump` and scoped tracing. Everything lives in
  `planning/tests/lib-test.sh`, needs no change to a test file, and is inert
  unless asked for.
- A failing test's temp root is **printed automatically**, and **kept in CI**
  while a local run cleans it up. CI has no re-run and no machine to come back
  to; locally you can just run it again.
- **Do not delete evidence a failure has not yet reported.**
  `docs/DEBUGGING-TESTS.md` records what that cost when `lib-test.sh` removed
  the root on any exit.
- When a suite's result is surprising, read its raw output rather than its
  summary, and read the **Skipped** and **Unconfigured** counts and lists in it,
  because a green run can still have run less than it seems. A test that calls
  `t_skip` ends as `SKIP`, not `PASS` (`BUGS.json` B268, fixed). A test that
  needs `PLANNING_CONTEXT_CACHE` and does not have it is `UNCONFIGURED` by
  design. Crate items become `UNCONFIGURED` when cargo is missing, and
  `REFUSE_UNCONFIGURED_CARGO=1` turns that into a `FAIL` (the `test` job sets it
  in its "Refuse an unconfigured plan-overview crate leg" step). What is still
  hidden: a test that skips one *check*
  inside itself and goes on to assert still prints `PASS`.
- **The pre-push gate shows why a crate failed, when the compiled
  `pre-push-check` is found (1.9).** `./pre-push-check.sh` runs
  `cargo fmt --check` and `cargo test` on each crate the change touches, and
  when one fails it prints, indented under the `FAIL` line, what the run said:
  for `cargo test`, up to 80 lines from its `failures:` section (each failed
  test's captured output), or the last 80 lines when there is none (a build
  error); for `cargo fmt --check`, the last 40 lines of its diff. Clippy
  prints its first 40 lines. The gate used to keep
  only the exit status, which left a failure that depends on the machine as a
  bare `FAIL cargo test: <crate>` that vanished on re-run. Read that output
  before re-running; a push that fails and then passes unchanged is a finding
  to chase (see 1.15), not luck. The bash fallback (`pre-push-check-lib.sh`,
  used when no compiled binary is found) discards cargo's output and prints only
  the `FAIL` line. The rest of the gate is in 1.17.

### 1.12a Test scratch space, and running one test

- **Where a test's scratch lives.** `run-tests.sh` makes one root per run under
  `$TMPDIR` (`ai-skills-tests.*`); every test child shares that run's `TMPDIR`,
  one `PLANNING_AGENT_TMPDIR` and `AI_SKILLS_TEST_RUN_ID`
  (`src/run-tests/src/main.rs`, `runner.rs`). **Per-test isolation comes from
  `planning/tests/lib-test.sh`:** sourcing it creates `$T_TMPDIR` for that test
  and exports it as `TMPDIR`, so a test run directly gets its own too. On Linux
  it is `${TMPDIR:-/tmp}/t.XXXXX`; on macOS it is always `/tmp/t.XXXXX`, because
  macOS's `$TMPDIR` is under `/var`, a symlink to `/private/var`, so a fixture
  repository there has two names and anything comparing paths disagrees with
  itself. **Compare canonical paths, not `$TMPDIR` spellings.** The root is
  removed on a local exit and kept on a CI one (1.12). CI's leak check
  (section 3) fails a leg on `ai-skills-tests.*`, `planning-agent` or
  `t.?????` left in `$TMPDIR` or `/tmp`; the last is a leaked `$T_TMPDIR`.
- **Writing a test.** Build the fixture in a `mktemp -d` under `$TMPDIR` (which
  `lib-test.sh` has already pointed at `$T_TMPDIR`) and clean it in a
  `trap ... EXIT`; a test that depends on a gitignored tree fails for the next
  contributor (`CODE-STYLE.md` section 12).
- **Bind sockets under `$T_SOCKET_TMPDIR`**, not `$TMPDIR`. It is `/tmp/s.XXXXX`,
  kept short on purpose: a unix socket path is capped near 104 bytes on macOS,
  and chromium (run by `mmdc`) adds about 50 for its profile, so a 75-character
  path failed `test-mermaid-accuracy` on the bash 3.2 leg where 62 passed. macOS's
  own `$TMPDIR` is about 49 bytes before anything of yours
  (`.agents/knowledge/github-ci-runners.md`,
  `.agents/knowledge/unix-sockets-across-platforms.md`).
- **No fixed ports or names** (1.15).
- **Running one test.** `./run-tests.sh --select-file FILE`, where `FILE` lists
  repo-relative shell tests or crate directories, one per line, as
  `./run-tests.sh --list-only` prints them. Or run the test directly with
  `bash planning/tests/test-x.sh`: that needs the built tree and
  `AI_SKILLS_BIN_ROOT` exported (1.9), and it runs without the resource wrapper
  and without the machine-wide lock. One crate is `cargo test -p <crate>`, which
  needs the nix shell (`nix develop`).

### 1.13 Compiled-binary wiring must survive a fresh, unbootstrapped checkout
- Wiring a script onto `plan_exec_compiled_binary_if_present` means sourcing
  `planning/scripts/plan-core-lib.sh` first — a **generated, gitignored**
  file that does not exist on a clone that has never run
  `build-plan-libs.sh`. **For a script that still has a plain bash body to fall
  back to**, guard the `source` + exec call on that file's own existence and fall
  through unconditionally to the body when it is absent, rather than let a bare
  `bash <script>.sh` fail outright on a fresh checkout. **Most scripts no longer
  have such a body:** most of `planning/scripts/*.sh` and `run-tests.sh` are
  shims that source the library unguarded and, with no compiled binary, exit 69
  ("no compiled binary found ... run ./setup-dev-env.sh to build it").
  `pre-push-check.sh` sources it unguarded too, and falls back to its bash
  implementation only once the library exists. On a fresh clone, run
  `./setup-dev-env.sh` before any of them. See
  `.agents/knowledge/compiled-binary-preference-fresh-checkout.md` for how
  this was found (real CI, not local testing — a stale local
  `plan-core-lib.sh` masks the bug) and which scripts are already fixed.
- A local `run-tests.sh` sweep cannot catch this class of bug by itself: the
  dev tree's own `plan-core-lib.sh` persists across sessions. Test the guard
  directly by moving the file aside and re-running the affected script.

### 1.14 `BUGS.json` and `TODO.json` are edited only on the `registers` branch
- `pre-push-check.sh` refuses any push that touches either register from a
  non-`registers` branch (1.17). A fix's resolution keys (`fix`, `verification`,
  `status`) go the same way as a new entry, after the code lands. The sequence:
  1. **Find the branch's checkout** with `git worktree list`. If `registers` is
     already checked out in another worktree (on the maintainer's machine it is,
     under `~/.config/tsch-ai-skills/worktrees/registers`), `git switch registers`
     in the main tree fails and that worktree is the one to use. Otherwise
     `git switch registers`, or `git switch -c registers origin/registers` if it
     is not local yet (never from `origin/master`: that drops an entry on
     `origin/registers` that has not landed). Worktrees are made as the
     `git-worktrees` skill says.
  2. **`cd` into that checkout** and `git pull --ff-only`. `registers-sync.yml`
     keeps the branch level with `master`, so it should be level.
  3. **Run the CLI there**, from `bin/<triple>/bugs` or `bin/<triple>/todo`
     (built by `setup-dev-env.sh`): `bugs add ...`, `todo add ...`, never a
     hand edit. **The CLI takes the register from the current directory:** `--file
     PATH` if given, else `BUGS_JSON` / `TODO_JSON`, else `./BUGS.json` /
     `./TODO.json`. Run in the main tree it edits the main tree's copy, which the
     gate then refuses. (`interactive-shell/TODO.json` is a second, unrelated
     queue.) Pass `--file "$PWD/BUGS.json"` when in doubt, then `bugs check`.
  4. **Commit and push `registers`.**
  5. **It lands by itself.** `registers.yml` runs `.github/registers-guard.sh` and
     fast-forwards `master` (section 3). A landing that fails says so in that
     run: a guard refusal prints `registers-guard: REFUSED: <reason>`, and a branch
     that is not a fast-forward of `master` is refused and needs a human to
     rebase `registers` onto `master`.
  - **A push from `registers` runs one check, and never enters nix** (1.17):
    every path changed since the merge base with `origin/master`, in the
    worktree or in the index, must be `BUGS.json` or `TODO.json`. Anything else
    fails and is named, and no other gate runs. It does not validate the
    entries: run `bugs check` (or `todo check`) yourself, and `registers.yml`
    checks ids and parents when the push lands. It fetches `origin master`
    first, so it needs the network.
  - **That is only in force where the checkout has the new script.** A
    `registers` checkout carries `master`'s files, so it gets this behaviour
    when it reaches `master`; as of 2026-09-21 it is only on `nextupdate`.
    Until then the hook on `registers` re-enters `nix develop`, and `master`
    lacks the `-fcommon` fix (`BUGS.json` B333, commit `ce08f36f`, only on
    `nextupdate` and `windows`), so on aarch64-darwin the dev shell cannot be
    built and the push dies there, before any check. That has stranded a
    worker; it is not a problem with the entry. Run `bugs check` yourself,
    then `git push --no-verify` (the hook's own text allows it "when you truly
    must"; `registers.yml`'s guard still gates the landing), or file from a
    machine where the dev shell builds. Do not "fix" it by putting other files
    on `registers`: both checks refuse anything but `BUGS.json` and
    `TODO.json`.
  - A worker's **uncommitted `BUGS.json` edit on another machine is not filed**
    until it is pushed. If someone else filed in the meantime the ids move (the
    CLI mints the next free one), so discard the local edit, `git pull --ff-only`,
    and re-file through the CLI, as was done for B365.
- **Always mint the id through the CLI (`bugs add`, `todo add`), never by
  hand.** The CLI resolves the next free id against the register's own
  current state; a hand-assigned id can silently collide with one already
  filed on `registers` that a feature branch's own stale local copy does not
  yet know about — found the hard way on 2026-09-16, when three
  hand-numbered bugs (meant to be B336-B338) collided with three real,
  differently-titled bugs already on `registers` at those exact ids. The
  branch-boundary check catches the write; it does not catch the collision.

### 1.15 A test never shares a machine-wide resource by literal
- A TCP or UDP port, a well-known path or a fixed name is shared with every
  *other* copy of the same test running on the machine: the pre-push gate
  beside a suite run, two worktrees, another agent. Ask the OS for a free one
  and pass it down (`free_udp_port()` in
  `src/chat-server-rs/tests/support/mod.rs`, `free_port()` in
  `src/chat-mcp/tests/mcp_flow.rs`); never write the number in the test.
- A "flaky" test that only fails while something else is running is a
  collision: look for the shared literal before calling it flaky or adding a
  retry. This is what a bare `FAIL cargo test: <crate>` from the gate that
  passes unchanged on the next push usually is.
- **A detached child is a shared resource too.** Git runs its automatic
  maintenance detached after a commit, so it outlives the tool that committed
  and creates and removes files under `.git`. `create-plan` therefore sets
  `gc.autoDetach` and `maintenance.autoDetach` to `false` in a repository it
  creates (never in an existing project's own). A test that snapshots, copies
  or deletes a directory a tool has just committed into is exposed to this
  unless the tool's git runs in the foreground.
- `./run-tests.sh` takes a machine-wide lock for the same reason
  (`AI_SKILLS_ALLOW_CONCURRENT=1` bypasses it and accepts the collisions).
  The gate's per-crate `cargo test` does not take it, so do not run the suite
  and the gate at the same time.
- The measurements behind both bullets, with the commands used, are in
  `.agents/knowledge/shared-machine-resources-in-tests.md`.

### 1.16 Windows is proved by CI legs, and it has its own conventions
- **A Linux run says nothing about Windows.** The supported Windows target is
  `x86_64-pc-windows-msvc` binaries with the bash skills run under Git for
  Windows' bash and its bundled coreutils, as a floor alongside bash 3.2
  (`CODE-STYLE.md` section 1, `README.md` and `DEVELOPMENT.md` point here for
  it). WSL2 is Linux, so it is covered by the Linux legs. The evidence is these
  CI legs:
  - `native`'s `x86_64-pc-windows-msvc` leg builds the workspace and runs
    `cargo test --workspace --all-targets --no-fail-fast`;
  - `test-windows` in `ci.yml` runs the shell suite in 4 shards, taking the
    scope job's selective test list like the Linux and macOS legs, and
    `test-suite-ok` ("Shell test suite (all shards) passes") requires it;
  - the two cygwin legs build the `x86_64-pc-cygwin` target and drive a real
    Cygwin bash and a real MSYS2 bash (the MSYS2 one links against MSYS2).
- **A quick verdict on one Windows problem needs the focus file.** Push to the
  `windows` branch (or run `windows.yml` by hand) with content in
  `.github/windows-focus.txt`: line 1 is passed to cargo, any further lines are
  a bash script run from the repository root, and only that runs. **Without the
  focus file `windows.yml` is no shortcut:** it runs fmt, clippy, the workspace
  tests and the unsharded shell suite, which took about as long as a whole
  `ci.yml` run (figures and run ids:
  `.agents/knowledge/windows-under-git-bash.md`). While the file has content
  both `windows.yml` jobs skip everything else, so delete it when the focused
  question is answered (`ci.yml` never reads it).
- **Conventions, each of which was a real failure** (symptoms and causes are in
  `.agents/knowledge/windows-under-git-bash.md`):
  - Git for Windows' `bash` and coreutils go first on PATH
    (`C:\Program Files\Git\bin`, `...\usr\bin`); System32's `bash.exe` is the
    WSL launcher.
  - Text is LF: `.gitattributes` pins it, and CI sets `core.autocrlf false`
    before checkout. `benchmark/results/` is left out of the checkout (paths
    longer than Windows allows).
  - Binaries carry `.exe`: use `planning_core::exe_name(name)` or
    `std::env::consts::EXE_SUFFIX`, never a bare `join(name)`. A test that
    needs a fake external command installs `tests/rust-support/script_stub.rs`'s
    shim (a compiled `.exe` that runs `bash <sibling script>`; a shebang script
    is not executable there). Its shell twin is
    `planning/tests/lib-script-stub.sh`, declared in `skill_files()`.
  - A path that crosses from bash to a native program, or into JSON, uses
    forward slashes (`C:/...`): `run-tests` hands scripts
    `PLANNING_AGENT_TMPDIR` that way, and shell tests use `t_is_windows`,
    `t_native_path`, `t_slashes` and `t_enable_symlinks` from
    `planning/tests/lib-test.sh` rather than their own `uname` checks.
  - Sockets: `planning-server`'s `transport.rs` is a Unix socket where there is
    one and loopback TCP with a nonce file where there is not. A socket read
    timeout is `TimedOut` on Windows, not `WouldBlock`
    (`chat-client-rs`'s `net::is_timeout` covers both), and a killed peer is
    `ConnectionReset`. End a connection on those lost-peer kinds only, never on
    every read error: that broader rule dropped live connections on macOS.
  - No mode bits on NTFS, no `ps -o`, no SIGHUP: gate the assertion or use the
    portable call (`Child::try_wait`; `verify-both-shells` installs a console
    control handler where unix installs signal handlers).

### 1.17 Git hooks and the pre-push gate
- `setup-dev-env.sh` sets `core.hooksPath` to `hooks/` (1.9), so two hooks are
  live in every dev clone:
  - **`hooks/commit-msg` refuses a commit message that has a line starting
    `Co-Authored-By: Claude` or `Claude-Session:`.** The maintainer asked for
    this: this repo's commits carry no such attribution. Agent harnesses add
    those trailers by default, so an agent's first commit fails until it drops
    them and commits again.
  - **`hooks/pre-push` runs `./pre-push-check.sh`.** Git's ref lines on stdin are
    discarded. A checkout that has no such file passes, so a branch that
    predates the gate is never blocked.
  - `git push --no-verify` skips the pre-push hook and `git commit --no-verify`
    the commit-msg one; CI gates the same things server-side regardless.
- **`./pre-push-check.sh --help` is the single source for the list of gates and
  their order**; it is not repeated here (it needs nix, or exits 69 before
  printing). `pre-push-check.sh` prefers the compiled `pre-push-check`
  (`src/pre-push-check`) and otherwise falls back to a bash implementation
  (`pre-push-check-lib.sh`) that prints less (1.12). What `--help` does not say:
  - It **fetches `origin master` first**, before it resolves the base or
    measures anything: master is the branch every change set is measured
    against, so the freshest one is used, and it is a fetch, never a merge into
    your branch. A failed fetch is itself a gate
    failure, because the change set would be measured against a stale ref. Retry
    before anything else: a dropped fetch is usually the network. With no
    `origin` remote it notes that and carries on. `PRE_PUSH_SKIP_FETCH=1` skips
    the fetch; it is for a test that drives the gate in a throwaway clone whose
    origin is a local path, and for diagnosis with no network, and the change
    set may then be stale.
  - **The base is the merge base** of `origin/master` and `HEAD` (falling back
    to `master`, then to the branch's upstream; with none of those, the worktree
    alone). The change set is the branch's commits plus the worktree and the
    index, against that base. So on a long-lived branch such as `nextupdate`
    nearly every crate counts as touched, and the per-crate `cargo fmt --check`
    and `cargo test` followed by the workspace clippy take several minutes.
  - **The registers-branch guard runs first and ends the run at once**: a change
    to `BUGS.json` or `TODO.json` from any branch but `registers` is refused
    (1.14). `PRE_PUSH_ALLOW_REGISTERS=1` accepts register changes already in
    flight; it is for transport, never for filing an entry.
  - **On `registers` it is a different, one-gate run**: no nix re-entry, no
    gate but the file scope (every changed path is `BUGS.json` or `TODO.json`,
    anything else fails). The
    compiled binary and the bash script both do this, and
    `tests/test-register-branch-gate.sh` and the crate's own integration tests
    pin it, including that a `nix` on `PATH` is never called.
  - It **regenerates `PORTABILITY.md` on every run**; that file is untracked
    (1.10).
  - **It re-enters the flake** (not on `registers`, above). Unless
    `AI_SKILLS_PREPUSH_IN_NIX` or `IN_NIX_SHELL` is set, it `exec`s `nix develop`, because git runs hooks with
    the caller's environment, which may carry a different cargo than the pinned
    one. Nix is mandatory (1.9): the gate exits 69 without it, and
    `AI_SKILLS_PREPUSH_IN_NIX` is the marker the re-entry sets on itself, not a
    way around nix. (The code skips the step on Windows, which has no nix; that
    is not a supported developer flow.)
  - **Exit codes:** 0 every gate passed; 1 at least one failed; 64 bad usage;
    65 not a git repository, or no base resolves and there is no branch diff;
    69 nix is needed and absent. **69 means no gate ran**: it is a refusal to
    start, not a pass.
  - `--full` also runs `./run-tests.sh`, the whole suite.
  - Export `AI_SKILLS_BIN_ROOT=$PWD/bin/<triple>` first (1.9), or the gate
    silently runs its bash fallback.
- The gate does not decide what needs judgement: the registers update, the plan
  validator and the role-drift tests stay with you. They are the change
  checklist in section 2 below, plus `planning/MAINTAINER.md` section 4 for a
  change to the planning skill.

## 2. Change checklist (minimum, per change)

1. Identify every consumer (parser/validator, other helpers, tests, manifest/map,
   `installer/src/50-manifest.sh`'s `skill_files()`, capsule copy, hash test).
2. Update shared logic in the library, keep the helper thin.
3. Add/update a regression fixture + test for the new behavior, including the
   actionable-error path.
4. If the change alters a flow that crosses more than two scripts, or adds/removes
   an artifact, update the affected diagram in the architecture doc in the same
   change (`CODE-STYLE.md` §11 picks the diagram form).
5. If a doc changed: keep the skill's `SKILL.md` small, update the phase/role docs
   and their references, regenerate any generated artifact (e.g. `REVIEWER.md`),
   and keep role/voice registries aligned; re-run the drift tests.
6. Register new files in `PACKAGE-MANIFEST.tsv`, `PACKAGE-MAP.tsv`, and
   `installer/src/50-manifest.sh`'s `skill_files()`; reflect benchmark-capsule
   dependencies in the capsule copy.
7. Run `bash -n`, `git diff --check`, and the bounded test suite (when one
   fails, `docs/DEBUGGING-TESTS.md` covers the evidence dump and breakpoints); run
   skill-specific drift/shape tests for any registry, voice, or generated-format
   change. For every change under `src/`, also `cargo fmt --check` and
   `cargo test` on each touched crate before pushing: CI runs fmt first, so
   unformatted or failing rust turns the CI legs red.
8. Update the registers, which nothing else will. A defect this change fixes is
   closed in `BUGS.json` with the commit and the mutation that proves it; a
   defect it *finds* and does not fix is added there rather than left in a commit
   message; queued work goes in `TODO.json`. Recipes are in the `bug-report` and
   `todo` skills. This is the one step with no gate behind it, so it is the one
   that gets skipped — and then the next reader has to reconstruct the change
   from its diff.
9. Commit as one coordinated, no-backwards-compat change. The message carries the
   *why* that does not belong in a comment (`CODE-STYLE.md` §12) and names the
   register entries it closes, so the two can be checked against each other.

### 2a. Adding a file

Each item names the rule's owner and the check that fails if you skip it.
- **Markers.** Every tracked file in the scanned trees (`planning`,
  `project-specifics`, `resource-limited-testing`, `brainstorm`,
  `post-implementation-review`, `todo`, `bug-report`, `installer`, `tests`, `src`,
  `.agents`) and these root scripts (and only these): `run-tests.sh`,
  `blast-radius.sh`, `generate-portability.sh`, `verify-both-shells.sh`,
  `setup-dev-env.sh`, needs `MODE: PROD|DEV` within its first 25 lines, as
  `# MODE: X`, `<!-- MODE: X -->`, `// MODE: X` or `/* MODE: X */`.
  `PACKAGE: PROD|DEV` goes **only** on what a compiler reads:
  `planning/scripts/lib/*/*.sh`, `src/*/src/*.rs`, `src/*/Cargo.toml`,
  `src/*/assets/*`. A `PACKAGE` marker on anything else fails as a stray one, so
  a crate's `src/*/tests/*.rs` carry `MODE: DEV` and no `PACKAGE`. Rule:
  `CODE-CONTRACTS.md` 10a. Check: `tests/test-mode-markers.sh`.
  - **A file that cannot carry a marker is exempt** (`exempt()` in that test):
    `*.json`, `*.jsonl`, `*.pub`, `*/FIXTURE-VERSION`, fixtures under
    `*/tests/fixtures/*`, `Cargo.lock`, `planning/PACKAGE-MANIFEST.tsv` and
    `PACKAGE-MAP.tsv`, `*.gitignore`, and compiled artifacts under
    `bin/<triple>/`. A new JSON file or binary needs no marker.
  - **For such a file, or one that must never ship, the tier is declared in the
    skill's `MODE-MANIFEST.tsv`** (only `planning/` and `interactive-shell/` have
    one): rows of `path<TAB>DEV|PROD|NEVER`, where a trailing `/` makes a
    directory prefix. Only the Rust installer reads it
    (`load_mode_manifest` in `src/installer/src/install.rs`); `skill_files()` does
    not.
- **A file under a skill directory that ships** is declared in
  `skill_files()` in `installer/src/50-manifest.sh`, in the **prod arm if it
  ships to end users and the dev arm if only a maintainer needs it** (a `dev`
  package is prod plus the dev arm; `RELEASE.md` explains the two, and
  `tests/test-mode-markers.sh` cross-checks each file's `MODE` marker against
  `skill_files()`). The pre-push gate runs
  `tests/test-skill-files-manifest.sh --declarations-only`, which fails on a
  tracked skill file that `skill_files()` does not declare. That is the whole
  rule for every skill but one, because `package.json` `files` already lists the
  skill's directory.
  - **For the `planning/` skill it is four places**, or
    `planning/tests/test-installer-manifest.sh` fails: `skill_files()`,
    `planning/PACKAGE-MANIFEST.tsv`, `planning/PACKAGE-MAP.tsv` (the manifest and
    the map are compared byte for byte, so **row order matters**) and
    `package.json` `files`. That test compares the planning manifest and map
    only; the manifest holds `planning/...` rows and nothing else.
  - **A new skill** also needs `SKILL_NAMES` and `SKILL_DESCRIPTIONS` in
    `installer/src/05-config.sh`, a `<skill>/requires.tsv` (empty when it has no
    runtime dependencies), a row in the `README.md` skills table and its
    directory in `package.json` `files` (`DEVELOPMENT.md`, "Adding or changing a
    skill").
  - A file that is a benchmark capsule dependency (`planning/scripts/*`,
    `SKILL.md`, `REVIEWER.md`) is also reflected in the capsule copy in
    `benchmark/planning/setup-benchmark.sh`.
- **A file directly under `.agents/`** stays untracked until you add a
  `!.agents/<file>` line to `.gitignore`, which ignores `.agents/*` apart from an
  allowlist (`.agents/knowledge/` and `.agents/profiles/` are already allowed).
- **A file that ships in the npm package** changes the packed set: see 1.10a for
  `.npmignore` and the size baseline.

### 2b. Adding a crate

- **Layout.** The root `Cargo.toml` is a virtual workspace over `src/*`, so every
  directory directly under `src/` must contain a `Cargo.toml`. A stray one (for
  example a gitignored `target/` left behind after a crate is deleted) breaks
  the whole workspace before anything builds, and `setup-dev-env.sh` exits 70
  naming it (`BUGS.json` B164). There is one root `Cargo.lock` only
  (`tests/test-rust-workspace-layout.sh`). `CODE-STYLE.md` section 1b and
  `rust-development-guidelines.md` are the rules for the crate itself.
- **A binary that skills or a release need** gets a row in **both**
  `setup-dev-env-lib.sh`'s `plan()` and `src/setup-dev-env/src/plan.rs`. The
  second is deliberately a second source of truth, and
  `src/setup-dev-env/tests/setup_dev_env_flow.rs` fails if they drift.
- **Markers.** Every `.rs` file under `src/*/src/` and every `Cargo.toml` needs
  `MODE: DEV` and `PACKAGE: PROD`; files under `src/*/tests/` need `MODE: DEV`
  only (2a).
- **A shipped binary** needs a row in its skill's `binaries.tsv` and a
  `skill_files()` entry (`rust-development-guidelines.md` section 6, checked by
  `tests/test-shipped-binaries.sh`).
- **CI needs no edit for a new crate:** `.github/ci-subjects.sh` treats a crate
  no subject claims as part of `planning_commands`, so it is built (the failure
  mode of that catch-all is a slower run, not a skipped crate).

## 3. CI map

Workflows live in `.github/workflows/`. `ci.yml` (named `ci`) runs on every push
to `master` and `nextupdate` and on every pull request; a superseded push is
cancelled (1.11). `test-suite-ok` is the single stable check name to require.

| workflow / job | what it proves | runs when | reproduce locally |
|---|---|---|---|
| ci `scope` ("Decide how much of the workspace this run must build") | nothing itself: it decides how much the other jobs build and which tests run (notes below), and runs the selector scripts compiled | always | `.github/ci-scope.sh --base origin/master`, `.github/ci-test-scope.sh --base origin/master`; their tests are in `.github/tests/` |
| ci `test` ("Test suite passes on ubuntu-latest / macos-latest (default bash, shard N)") | the deterministic suite passes with each OS's default bash and leaks no temp files; 2 OS × 4 shards, 45 min timeout | always | `./run-tests.sh --shard N/4` (add `--select-file FILE`) |
| ci `test-bash32` ("… macOS system bash 3.2 (portability floor, shard N)") | the suite runs on the bash 3.2 floor of `CODE-STYLE.md` section 1; 4 shards, 45 min | always | `bash32-run-tests` in the dev shell (bash 3.2 on this OS); BSD userland differences only show on the macOS legs |
| ci `test-windows` ("… windows-latest (Git bash, shard N)") | the shell suite under Git for Windows' bash (1.16); 4 shards, 60 min | always | none from Linux; `windows.yml` is the quick loop |
| ci `test-suite-ok` ("Shell test suite (all shards) passes") | every shard of `test`, `test-bash32` and `test-windows` succeeded; `if: always()` so a failed shard fails it instead of skipping it, which a required check would treat as passing | always | — |
| ci `native` ("<target> builds and runs every subject in scope") | on each of 5 native runners (x86_64 and aarch64 linux-musl, x86_64 and aarch64 apple-darwin, x86_64-pc-windows-msvc): `cargo fmt --all --check`, workspace clippy with `-D warnings`, `cargo test --workspace --all-targets --no-fail-fast`, then each in-scope subject builds and its binary launches (musl builds are checked to be static) | scope is not `none` | the same three cargo commands on the host triple |
| ci `cygwin`, `msys2` | `x86_64-pc-cygwin` (tier 3, nightly `-Z build-std`) builds and drives a real Cygwin bash and a real MSYS2 bash | scope is not `none` | none from Linux |
| ci `rust-artifact-manifest` | the per-target sha256 manifest of the planning command builds is produced and verifies | the planning commands are in scope | — |
| ci `shellcheck` | no warning-level shellcheck finding in tracked `*.sh` (minus `benchmark/results/`) plus the five generated `plan-*-lib.sh` | always | `shellcheck -s bash --severity=warning` on that set |
| ci `mermaid-render` | every fenced mermaid block parses and renders: syntax only, `planning/tests/test-mermaid-accuracy.sh` owns accuracy | always | `mmdc` is in the dev shell |
| `render-artifacts.yml` | the `plan-overview` binary builds and runs natively on 5 targets | push to `master`/`nextupdate`, pull request, manual | `cargo build -p plan-overview` |
| `windows.yml` | the quick Windows loop (1.16) | push to `windows`, manual | — |
| `registers.yml` | a push to `registers` passes `.github/registers-guard.sh` and fast-forwards onto `master` | push to `registers` | 1.14 |
| `registers-sync.yml` | keeps `registers` level with `master` | push to `master` | — |
| `release-installer.yml` | attaches the per-platform installer assets to a GitHub Release | release published, or manual | `RELEASE.md` |

What the table cannot say:
- **Which checks gate a merge.** Branch protection lives in GitHub, not the
  repository. Read on 2026-09-21 with
  `gh api repos/tschallacka/ai-skills/branches/master/protection/required_status_checks --jq '{strict, contexts}'`,
  `master` requires exactly these: "Decide how much of the workspace this run
  must build" (`scope`), "Every mermaid diagram renders ...", the five
  "plan-overview `<target>`" jobs of `render-artifacts.yml`, "Shell test suite
  (all shards) passes" (`test-suite-ok`), and a ninth, "install.sh matches
  installer/build.sh output" (nine in all). **That ninth check is retired:** no workflow has a
  job of that name, so it can never report and a pull request cannot go green
  without an admin bypass (`enforce_admins` is off, `strict` is off, no reviews
  are required). Removing it is the repository owner's job, tracked as `TODO.json`
  T149, and it blocks nothing yet, because nothing is merged to `master` (as of
  2026-09-21). Everything else is advisory, not required: `shellcheck`, `native`,
  `cygwin`, `msys2`, each shard job on its own (only the aggregate is required),
  `test-windows`'s shards likewise, and `windows.yml`. `nextupdate` has no
  protection. **A required check is matched by its job's `name:`, so renaming
  `test-suite-ok`'s name or a `render-artifacts.yml` job silently orphans the
  requirement:** change the protection setting in the same step.
- **`scope`.** A pull request gets a selective scope: `ci-scope.sh` builds the
  changed crates plus their dependents (a closure bigger than a quarter of the
  workspace, at least 5, goes full) and `ci-test-scope.sh` picks the shell tests.
  **A push to `master` or `nextupdate` is always full** (`--push-to`): on a push to
  `master`, `HEAD` is `origin/master`, the diff was empty, and the selector once
  reported `none` and turned master green having compiled nothing. Every error
  path also returns full (no merge base, shallow clone, `cargo metadata`
  failing). `none` skips `native`, `cygwin` and `msys2` but never the shell shards.
- **`COVERS:`.** A shell test opts into selection with a `# COVERS: <path> ...`
  comment (that exact hash-comment form) within its **first five lines**; a
  marker on line 6 or later is never seen (`src/ci-test-scope/src/covers.rs`). A
  changed path hits an entry when it equals it or is under it. **A test with no
  marker always runs**, so add one only when you can state what the test
  covers. **A marker whose entries match nothing in the change set excludes the
  test** from that run, an empty entry list included, so a wrong path silently
  stops a test running on pull requests.
- **`SHARD_TOTAL`** is a literal in `test`, `test-bash32` and `test-windows`
  (4 each) and must equal the length of that job's `shard` array. A wrong count
  silently under-covers, because every shard still passes; `tests/test-run-tests-shard.sh`
  proves only the partition arithmetic.
- **Every shard job** builds with `SETUP_DEV_ENV_IN_NIX=1 ./setup-dev-env.sh`
  and then pins `AI_SKILLS_BIN_ROOT` (1.9).
- **Temp-leak check.** `test` and `test-bash32` (not `test-windows`) fail the
  leg if `run-tests` leaves `ai-skills-tests.*`, `planning-agent` or `t.?????`
  under `$TMPDIR` or `/tmp`.
- **Registers.** An entry filed on `registers` (1.14) is checked by
  `.github/registers-guard.sh`: every changed path is `BUGS.json` or
  `TODO.json`, no id repeats, every parent resolves. Git cannot see a duplicate
  id, which is why this runs at all. It then fast-forwards onto `master`, never
  merges, using the `REGISTERS_PUSH_TOKEN` secret because master's branch
  protection blocks the default `GITHUB_TOKEN`. `registers-sync.yml` keeps
  `registers` level with `master` using `github.token`, and never forces: if
  `registers` holds a filed entry `master` lacks, it stops and leaves it.
- **Reading a red run:** `ci-failures <branch> --raw DIR` (the `ci-failures`
  skill) extracts each failed job's failing test output. The Windows legs also
  upload their logs as artifacts, which is the easiest way to read Windows
  output: `windows-shell-suite-log-shard-N` from `ci.yml`, and
  `windows-test-log` and `windows-shell-suite-log` from `windows.yml`.