<!-- MODE: PROD -->
# Repository Handbook — behavior rules & change checklist

**Audience: agents and maintainers.** This is the repo-wide, agent-facing
contract: the rules that govern how every skill and helper in this repository
is built and changed. The planning skill's own architecture and format details
live in `planning/MAINTAINER.md` and `planning/MAINTAINER-STYLE-CONTRACT.md`;
this file holds what applies to the repository as a whole.

## 1. Behavior rules

### 1.1 No backwards compatibility
- A changed command/format is a **clean break**. No aliases, legacy modes, or
  inferred defaults. Old forms fail loudly.
- Coordinated migration: update producer, parser/validator, fixtures, tests,
  manifest/map, `install.sh skill_files`, and the hash test in the **same**
  change.

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
- Run `setup-dev-env.sh` once after cloning. It builds every crate under
  `src/` for this machine's target triple into ONE `bin/<target triple>` at the
  repository root — the directory the plan helpers resolve as the shared binary
  home (`plan_bin_dir`) — rather than a `bin/` inside each skill.
- Only the host triple is built locally; cross-building the other targets is
  what a release (`installer/build-release.sh`) and CI do. A green run on an
  unbuilt tree is not evidence about the code a user gets — the compiled path a
  target actually runs is never exercised locally.
- `rjq` is invoked by name, so the root `bin/<triple>` must be on PATH for the
  helpers to find it (or the tree falls back to whatever `rjq` the machine has).

### 1.10 Generated files are CI's job, not the repo's
- Every binary and compiled output is built by a CI runner and delivered as a
  release artifact. The repo carries no generated files — nothing
  machine-produced is committed, ever.
- This is absolute, and it names the files that are tracked today and must
  leave: committed binaries, compiled `plan-*-lib.sh` outputs, and generated
  Markdown such as `PORTABILITY.md` and `REVIEWER.md`. Each moves to a CI build
  that publishes the artifact, and its in-repo copy is removed in the same
  coordinated change.
- Until a file's migration lands, its existing gate keeps running and a stale
  generated file still fails it. The rule adds "not tracked" as the required
  end state; declared-but-unbuilt is the legal resting state on disk.
- Consumers that read a generated file from the working tree — the installer,
  npm pack, test harnesses, `blast-radius.sh`'s freshness checks — must be
  reconciled to fetch the artifact from the release or publish pipeline in the
  same change that untracks the file.
- **`install.sh` is the one standing exception, and it stays committed.** It is
  fetched and run standalone (`curl … | bash`) and is the npm `bin`, so at
  runtime it has no siblings to source and no pipeline to fetch it from — the
  artifact *is* the entry point. Read "nothing machine-produced is committed"
  as "except the one artifact whose whole purpose is to be downloaded on its
  own".

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
- **So do not push while a run you need is still queued.** The queue here can
  take around fifteen minutes merely to *register* a run, and anything pushed
  inside that window destroys the previous result. This cost two experiments in
  one day: a diagnostics run that would have printed a macOS bind errno, and a
  rebase's verification. Batch the pushes, or hold them until the run completes.
- The distinction that matters: you lose the result **only when the earlier
  commit was the one being tested**. A superseded run of code you have already
  replaced is no loss at all.
- macOS runners are the scarce resource and set the critical path. A readiness
  or retry budget written for a Linux runner will be too tight there — the host
  is a shared, oversubscribed VPS that pauses for other tenants.

## 2. Change checklist (minimum, per change)

1. Identify every consumer (parser/validator, other helpers, tests, manifest/map,
   `install.sh skill_files`, capsule copy, hash test).
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
   `install.sh skill_files`; reflect benchmark-capsule dependencies in the
   capsule copy.
7. Run `bash -n`, `git diff --check`, and the bounded test suite; run
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