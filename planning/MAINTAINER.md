<!-- MODE: DEV -->
# Maintainer guide — architecture, relations & behavior rules

**Audience: skill maintainers and developers only. This is NOT part of the
agent-facing planning instructions.** Do not point a planning agent at this
file for execution guidance; it exists to keep the skill coherent as it
evolves. It complements `MAINTAINER-STYLE-CONTRACT.md` (document-format rules);
this file covers the architecture, the relations between artifacts, and how
the maintainer must behave going forward.

## 1. The artifact map (what references what)

| Artifact | Role | Referenced by / depends on |
|---|---|---|
| `SKILL.md` | Lean index + shared contract; routes an agent to the right doc for the task. | Read by any agent deciding to use the skill. Must stay small. |
| `PLANNING.md` | Phase doc: create → decompose → goals → steps → review/validate. | From `SKILL.md` when the task is planning. |
| `EXECUTION.md` | Phase doc: resume → bounded context → monitor steering → coordinator-subcontractor model → helper mutations → env evolution. | From `SKILL.md` when the task is execution. |
| `ROLES.md` | Routing index + persona doc matrix (mirror of `role-context.sh` `role_docs()`) + per-role reader allow-list; no full role table (that lives in the contract). | From `SKILL.md`; routes to `roles/*.md`; scope matrix mirrors `role_docs()` in `role-context.sh`. |
| `roles/planning.md` `roles/execution.md` `roles/cleanup.md` | Phase-scoped role rosters. | From `ROLES.md` per phase. Each lists only that phase's roles. |
| `REVIEWER.md` | Reviewer contract (generated on demand by `scripts/generate-reviewer.sh` from `SKILL.md`, pinned to its hash; never committed, §2.16). | From review docs; loaded by reviewers; consumers generate it when absent. |
| `MAINTAINER-STYLE-CONTRACT.md` | Canonical roles master + benchmark-domain roles + document-format contract (internal). | Maintainer only; phase docs reference ids, never redefine. |
| `MAINTAINER.md` | This file (internal). | Maintainer only. |
| `ARCHITECTURE.md` | The five cross-script flows as mermaid diagrams (plan lifecycle, role handoff and gates, script layering, the validation state machine, plan-directory state files) plus the recorded contradictions and dead ends (internal). | Maintainer only; not shipped, not in the manifest/map, not installed. Read it before changing a flow that crosses more than two scripts. |
| `scripts/role-context.sh` | Gatekeeper: machine source of the persona registry (`ROLES=()`) + per-role scope (`role_docs()`); prints a role's docs + voice preamble (paginated). `--list` open (id/name only); `--paths` maintainer-only; content reads FAIL CLOSED without a valid `ROLE_ID`. Sourcing guard exposes registry + resolvers for reuse. | Reads `SKILL.md`, `ROLES.md`, `roles/*.md`, `REVIEWER.md`, `MAINTAINER-STYLE-CONTRACT.md`, `roles/VOICES.md` (voice preamble). |
| `roles/VOICES.md` | Per-role voice/stance identity preamble, keyed by canonical `ROLE_ID`, byte-budgeted (≤512 B). | Injected by `role-context.sh` `voice_for()`; asserted by `test-voice-artifact-drift.sh` + `test-persona-drift.sh`. |
| `scripts/monitor-read.sh` | Maintainer-only monitor reader: bounded supervision frames, pull-on-exception, grant log (case+command). | Reads frames written by `supervision-frame.sh`; identity-gates to maintainer (fail closed). |
| `scripts/supervision-frame.sh` | Bounded supervision-frame emitter + grant log (case+command, never reasoning); footer-overwrites. | Written by each subagent at end; read by `monitor-read.sh`; asserted by `test-supervision-frame.sh`. |
| `scripts/plan-context.sh` | Bounded plan-context reader (init/read/check/refresh/checkpoint). | Gates per `ROLE_ID` via `plan-context-lib.sh`; context IDs tagged (`plan`, `goal:<id>`, `step:<goal>/<step>`, `unit:WNN`). |
| `scripts/plan-context-lib.sh` | Shared bounded plan-context cache helper; owns per-role reader composition (`context_role_gate` / `context_role_reader_composition`). | Reads registry via `role-context.sh`; gates `plan-context.sh` per `ROLE_ID`. |
| `scripts/plan-document-lib.sh` | Shared document helpers (sections, paragraphs, replace, guard). | All mutating helpers. |
| `scripts/plan-reconcile-lib.sh` | Shared reconciliation (coverage prune, owned-work-units rewrite, progress rebuild). | `add-work-unit.sh`, `remove-work-unit.sh`, `update-adversarial-review.sh`. |
| `scripts/validate-plan.sh` | Plan gate entry point: flags, the skill-root path, the single EXIT cleanup, and the ORDER of the 13 validation passes. Owns no check itself. | Sources every `validate-plan-*-lib.sh`; the pass order is load-bearing (see the file's docblock). |
| `scripts/validate-plan-common-lib.sh` | `fail`/`warn` finding vocabulary, `trim`, `require_heading`, `get_single_field` (returns via the `field_value` global). `warn` never touches `errors` — the `--complete`-promotes-WARN-to-FAIL pattern depends on it. | Required by every other `validate-plan-*-lib.sh`. |
| `scripts/validate-plan-docs-lib.sh` | Existence gate, plan-description headings, `UI affected` verdict, adversarial-review gate and mirror, inventory section shape, hand-edit damage (helper-flag text, duplicate `§` labels, `$script_dir` fragments). | Publishes `ui_affected` and the `plan_docs` list the placeholder and stale passes iterate. |
| `scripts/validate-plan-placeholders-lib.sh` | Registered-template-token sweep; the registry surface (authored/generated) decides WARN vs FAIL. | Reads `planning/placeholders.json` via `skill_root`. |
| `scripts/validate-plan-stale-lib.sh` | `--stale` phrase sweep, including the `*-testing.md` companions; bundled case-count and byte-identical phrase list. | Registers its temp phrase file with the entry script's `cleanup_files`, never its own EXIT trap. |
| `scripts/validate-plan-inventory-lib.sh` | Work-unit row parser and all row rules, definition-of-done cross-links, dependency-cycle walk, and the (KNOWN DEAD) proof-coverage rule. | Owns the `unit_*` / `goal_units` data model every later pass reads. |
| `scripts/validate-plan-ui-lib.sh` | UI user stories, browser run caches, story status vocabulary, `bugs.md` linkage. | Runs only when `ui_affected` is `yes`. |
| `scripts/validate-plan-goals-lib.sh` | `goal.md` headings, the testing-requirement table, goal-size band and exception, step-file/inventory agreement, step-file naming. | Only writer of `goal_testing_required`. Reads `planning/goal-tables.json` via `skill_root` for the sections whose first column may be `yes`/`no`; a table under any other heading is hand-edit damage and fails. |
| `scripts/validate-plan-serve-lib.sh` | The "the application still serves" WARN for a goal that changes module state, schema, or configuration. | Reads `planning/state-change-registry.json` via `skill_root`. |
| `scripts/validate-plan-commands-lib.sh` | Command-literal detector (rules 1-8) against the plan's `commands.json`. | Reads `planning/never-executable-extensions.json` via `skill_root`; registry maintained by `register-command.sh`. |
| `scripts/validate-plan-propagation-lib.sh` | `--complete` progress gate plus propagation (a) unowned edit targets, (c) verifier reachability, (c2) companion references, (d) unverified graph leaves, (e) §9.x roster vs inventory. | Runs only when `--propagation` (the default) is on. |
| `scripts/*.sh` | Thin, single-purpose helpers. | Source `plan-document-lib.sh` (+ `plan-reconcile-lib.sh`). |
| `PACKAGE-MANIFEST.tsv` / `PACKAGE-MAP.tsv` | Ship manifest / source-destination map. | Every installed file must be registered here. |
| `install.sh` `skill_files()` | Installer file list. | Must match manifest + map. Every name must exist on disk and every tracked skill file must either be listed or parked in `../installer/unshipped-planning-files.txt` — asserted by `../tests/test-skill-files-manifest.sh`, which also runs as npm `prepack`. |
| `../installer/unshipped-planning-files.txt` | Planning files no install delivers, awaiting a ship-or-not decision. | A ratchet: entries leave by being registered for shipping. Adding one is a decision, and a stale entry fails the test. |
| Benchmark capsule copy (`setup-benchmark.sh`) | Copies a fixed set into the worker capsule: `SKILL.md`, `REVIEWER.md` (generated into the capsule when absent), `scripts/`, and the UI reference doc. | New files under `scripts/` and changes to `SKILL.md`/`REVIEWER.md` must be reflected here; `ROLES.md`, `roles/*`, `VOICES.md`, and `MAINTAINER-STYLE-CONTRACT.md` are NOT copied into the capsule. |
| `../verify-both-shells.sh` | Runs the suite on the working tree under the local bash and the bash 3.2 floor, in a linked worktree in `TMPDIR` so editing can continue. Prints each failing test's own output. | Sweeps its own leftover worktrees; never place one under the repo, or the filesystem scans land machine-specific paths in generated artifacts. |
| `../blast-radius.sh` | Integration-safety report over a change set: freshness of generated artifacts, missing manifest rows, base drift, and the couplings in `coupling.tsv`. Not a correctness check and not shipped. | Reads `coupling.tsv`; runs `installer/build.sh --check`, `generate-portability.sh --check`, `test-reviewer-projection.sh`. Asserted by `test-blast-radius.sh`. |
| `../coupling.tsv` | The couplings a change must honour, as data rather than prose: glob, level, consequence, check. | Read by `blast-radius.sh`. Add a row when a new generated artifact or registry appears. |
| `../BUGS.json` / `../TODO.json` | The defect register and the work queue, written by this repo's own `bug-report` and `todo` skills. Tracked, so they survive a machine. | Update them in the same change (checklist step 8). No gate enforces this — nothing can tell that a commit resolved a defect — so the checklist is the only mechanism. |
| `../CODE-CONTRACTS.md` | Cross-script behavioural contracts: section shapes and their sole writer, irreversible-last, advisory vs gate, generated artifacts, registry-plus-gate, and human-followable documents. Each entry names the incident behind it. | Read with `CODE-STYLE.md` before changing a helper. Contract 1 is enforced by `document-sections.json` + `test-document-sections.sh`. |
| `../planning/document-sections.json` | Every section-form target and its shape (narrative, fields, hybrid, table). | Read by `test-document-sections.sh`; the runtime guard lives in `plan_replace_section`. |
| `.plans/` | Transient plan storage (gitignored). | Created by `plan-root.sh`/`create-plan.sh`; `.env` regenerated by `plan-env.sh`. |
| `benchmark/planning/` | Benchmark harness; owns domain roles (Oracle/Pythia, Analyzer/Eve). | Not part of the end-user lifecycle. |

## 2. Behavior rules for the future

### 2.1 No backwards compatibility
- A changed command/format is a **clean break**. No aliases, legacy modes, or
  inferred defaults. Old forms fail loudly.
- Coordinated migration: update producer, parser/validator, fixtures, tests,
  manifest/map, `install.sh skill_files`, capsule copy, and the hash test in
  the **same** change. Run the plan validator and installer-manifest check.

### 2.2 Small, scoped, single-source docs
- `SKILL.md` stays a lean index/contract. Never let it regrow into the
  monolithic document it replaced.
- Agents read only the doc for the task they are doing. Phase scoping prevents
  "future knowledge" leaking into a planning/execution/cleanup context.
- Shared facts live in exactly one place: the contract in `SKILL.md`, the
  canonical roles in `MAINTAINER-STYLE-CONTRACT.md`. Phase docs and role docs
  **reference, never duplicate**.

### 2.3 Proactive, reconciling tools
- When a mutation happens, the tool reconciles every reference automatically —
  coverage rows, Owned-work-units, "Depends on", step/testing files, and
  progress trackers. Do not make an agent issue a follow-up call the tool
  knew it needed.
- Keep helpers **small**; put shared logic in library files
  (`plan-document-lib.sh`, `plan-reconcile-lib.sh`).

### 2.4 Deterministic command contracts
- Every subcommand has one fixed, documented positional signature. An explicit
  `document-id` where applicable. No positional overloading, no value-sniffing.
- Every mutating helper has `--help` (exit 0, concise) and **actionable
  errors**: state the problem and what the agent can do to resolve it.

### 2.5 Identity-gated capabilities
- Revealing capabilities (e.g. `--paths`) are gated by caller role (`ROLE_ID`);
  `--list` is deliberately open (id/name only). Default print mode only ever
  emits the requested role's own docs.
- Content reads FAIL CLOSED: an unset or unknown `ROLE_ID` is a hard refusal
  with a `FAIL-CLOSED identity` message — the worker is denied a persona and
  must be respawned. Only `--list` is identity-free.
- Shell gates are **advisory, not a security boundary**; the agent framework
  confines the process. Document that.

### 2.6 Plans are transient work orders
- Plans are not fixtures. `.plans/` is gitignored; `.env` manifests are
  regenerable by `plan-env.sh` and never committed.
- To pin one plan under audit, use the `.gitignore` negation
  (`!.plans/<plan>/` + `!.plans/<plan>/**`), not force-tracking everything.

### 2.7 Roles are a canonical registry
- Canonical ids/names are assigned once (Alex, Benny, Chris/Christian/Christoph,
  Dana, Frank, Willie, Felix, Pythia, Eve). Names are meaningful only
  alongside the id; never repurpose a name.
- Benchmark-domain roles (Oracle/Pythia, Analyzer/Eve) have their full
  authority defined in the maintainer contract; they appear in the `ROLES.md`
  persona matrix only because scope-doc shipping requires every `ROLES=()` id to
  be present, but their authority is not defined there.

### 2.7a `role-context.sh` is dual-natured

- It is a CLI **and** a sourceable registry: the sourcing guard stops the CLI
  main flow when the file is sourced, so `plan-context-lib.sh` can reuse
  `resolve_id()` without the arg parsing, usage and exit firing in the caller.
- `ROLES`, `resolve_id`, `canonical_name`, `role_docs`, `list_roles`,
  `voice_for` and `can_access` are the public surface of the sourced form.
  `ROLES` keeps its UPPER_CASE name and none of these carry the `plan_` prefix
  CODE-STYLE.md section 7 asks of a sourced file; renaming any of them is a
  cross-file change. Everything script-local to the CLI half is lower-case.
- Resolution accepts the canonical id and the canonical name
  (case-insensitive) plus id/name aliases (`willie`/`maintainer`,
  `pythia`/`oracle`, `benny-02` → `benny`). Unset or unknown `ROLE_ID` is a hard
  refusal and the worker is denied a persona; `--paths` is maintainer-only.
  Shell gates are advisory, not a security boundary — the agent framework is
  what confines the process.
- `ROLES=()` and `role_docs()` are the machine source of the persona registry
  and per-role scope; the `ROLES.md` matrix is a maintained mirror, and
  scope-doc shipping is enforced by `tests/test-persona-drift.sh`.

### 2.8 Review protocol invariants
- Protocol 1.4.2: Reviewer A (`christian`) is handoff-only, never approves;
  Reviewer B (`christoph`) is sole approval authority. Adversarial review is
  done by a fresh secondary agent with no access to the planner's conclusions.
- Validate before creating progress trackers; a plan is not ready until the
  review is approved and validation passes.

### 2.8 The compiled libraries

`plan-core-lib.sh`, `plan-document-lib.sh`, `plan-table-lib.sh`,
`plan-progress-lib.sh` and `plan-crypt-lib.sh` are compiled by
`build-plan-libs.sh` from `scripts/lib/<group>/*.sh`, one function per file. Do not edit the compiled
files; edit the function file and re-run the build. `--check` reports a stale
library, and `test-plan-libs-build.sh` runs it.

- `plan-document-lib.sh` remains the façade. Its `99-facade.sh` sources the other
  four plus `plan-map-lib.sh` and `plan-inventory-lib.sh`, so the 40-plus
  scripts that source that one path keep getting every symbol. Verified as a
  symbol-set comparison, not by inspection.
- `crypt` is the fifth group and was cut out of `core`, not invented alongside
  it. The digest chain, the fix-key derivation, the target-triple lookup and the
  CSPRNG went in; what stayed in `core` is how a script fails, guards and writes.
  The split happened because `core` reached 506 lines against the 500-line cap
  CODE-STYLE.md section 3 sets, and the cap is a ratchet: raising one to fit new
  work is the change that makes every later violation invisible.
- Each compiled library begins with a `PLAN_<GROUP>_LIB_LOADED` guard, because
  the façade sources its siblings and a script may source both. Without it a
  second source would re-run `00-state.sh` and drop the registered temp files.
- Group membership is the directory. Moving a function between groups is a `git
  mv` plus a build, and the symbol set must not change.
- Their committed form is grandfathered, not endorsed: §2.16 makes untracking
  them the destination, with the build and the artifact moving to CI.

### 2.8a Fix-key derivation

- `mint-fix-keys.sh` parses the five-column Findings table
  (`ID | Missing or over-broad item | Required plan change | Status | Work unit`)
  and writes one hex key per (finding, work unit) row into `fix-keys.json`,
  derived as `SHA-256(secret || "<session_id>|<finding>|<work unit>")` with the
  64-hex-char secret first (T16 retired HMAC). Rows with no work unit carry no
  key. Keys minted under the retired HMAC scheme do not verify; re-mint.
- The derivation itself is `plan_fix_key` in `plan-crypt-lib.sh`, called by both
  the minter and the verifier. It used to be a copy in each script under a
  comment saying the two must stay byte-identical, and `test-duplication-ratchet.sh`
  held them to it; one definition is the mechanism that comment was asking for.
- No single hash tool is a hard requirement. `plan_sha256_hex` walks the shipped
  `plan-crypt` binary first, then the GNU tool, then the BSD one, refusing with
  69 only when none exists. `openssl` was the last rung and is gone from the
  chain and from `requires.tsv`: a static binary asks the target machine for
  nothing, so shipping one *lowered* the declared requirement. The binary is
  opportunistic, found the way `chat-server.sh`'s `rust_bin()` finds its own —
  `PLAN_CRYPT_BIN`, then `PATH`, then `planning/bin/<target triple>/plan-crypt`
  and `planning/bin/plan-crypt`. Per-target artifacts are CI-delivered and
  untracked (§2.16); a local `planning/bin/<triple>` path exists only after a
  local build. A pin naming a file that does not exist is a
  refusal rather than a fall-through, which is how a test takes the compiled
  rung out of the picture.
- Two implementations of one algorithm can disagree, so they are pinned to each
  other rather than trusted: `tests/test-plan-crypt.sh` asserts the compiled and
  shell rungs produce identical hex across the empty string, the 55/56/64-byte
  padding boundary and the exact strings fix keys are derived over. That
  equivalence is what lets a key minted before the binary existed keep verifying.
- `ensure_session_secret` is the production creation path for the session
  secret; tests seed the secret directory themselves. Both the session id and
  the secret come from `plan_random_hex`, which is the OS CSPRNG or a refusal
  — there is deliberately no improvised third arm. The one that existed built
  the id from the process id, the whole second and the shell's own weak PRNG,
  and that value keys the gate (B87).
- `add-fix-claim.sh` is the only writer of `fixes.md`, which had five readers
  and none. It gates on the same Findings table `mint-fix-keys.sh` and
  `verify-fix-keys.sh` read, so a claim it accepts is a claim the verifier
  gates; reading `fix-keys.json` for the pair list instead would let the three
  disagree. It checks the key by membership in `fix-keys.json`, never by
  derivation: derivation needs the session secret, and a fixer able to read that
  secret could mint its own keys.

### 2.9 Per-role reader composition
- `role-context.sh` (persona scope) and `plan-context.sh` (bounded plan content)
  are two distinct gates; there is **no global composition rule**, only a
  per-role allow-list (`plan-context-lib.sh` `context_role_reader_composition`).
  `installer`/`oracle`/`eve` read no plan content; all other roles are capped
  per role.
- Keep that function and the `ROLES.md` reader allow-list in sync; they must
  not drift from `role_docs()`/`ROLES=()`.

### 2.10 Supervision monitor is maintainer-only, pull-on-exception
- Willie reads only each subagent's bounded supervision frame
  (`supervision-frame.sh`, ~2048 B, footer-overwrites), via `monitor-read.sh`,
  pulling deeper only when a frame is `escalated`/`out-of-bounds`/`blocked`.
- The grant log records case + handed command, **never reasoning**. Frames and
  grants fail closed to any non-maintainer.

### 2.11 Which repository owns a plan's history

`create-plan.sh` decides this once, at creation:

| Situation | Repository used |
|---|---|
| Plans root is git-excluded (a project's `/.plans` in `.gitignore`) | Its own repo at the plans root, so the whole plans tree is versioned and cross-plan `plan-content.sh diff` can walk up. Re-initialised when the root's `.git` is missing. |
| Plans root is outside any repo | Same: its own repo at the plans root. |
| Plan sits inside an already-versioned tree | That tree; no new repo. |
| Explicit path outside any repo | Its own per-plan repo. |

### 2.12 Command-literal detection is registry-driven and language-agnostic

Every command literal in a step file or testing companion must be registered in
the plan's `commands.json` (seeded empty by `create-plan.sh`, maintained with
`register-command.sh`), so the "when" context travels with the command instead
of being lost when steps are copied. `validate-plan-commands-lib.sh` decides
what is a command literal with ordered rules, no language table:

A span is a **candidate** when any of these holds —

1. its first token is a registered command's first token, or a universal core
   word;
2. it resolves on disk to a non-directory with the executable bit (the same
   question a shell would ask);
3. its last segment sits directly under a bin-like directory (`bin/`, `sbin/`,
   `.bin/`, `Scripts/`, covering `vendor/bin/` and `node_modules/.bin/`).

Rules 1–3 are the only entry points: arguments strengthen a qualifying span but
never qualify one on their own. It is **disqualified** regardless when —

5. its last segment carries a never-executable data/markup extension (the
   rjq-matched list in `never-executable-extensions.json`);
6. it ends in a `:line` / `#Lnn` citation;
7. it is route- or prose-shaped (a leading `/` without a bin-like segment whose
   first token is not command-shaped, or a leading-`/` argument after a
   non-command-shaped token, e.g. `GET /health`);
8. its first token resolves to a directory.

The vocabulary is the registry itself: each registered command's first token
teaches the detector that tool word, so a Python plan registering `pytest -q`
makes `pytest` a word with no language table to maintain. Findings WARN
mid-draft and FAIL under `--complete`, like authored placeholders.

### 2.13 Placeholder detection is literal registry membership

`planning/placeholders.json` is the exact, finite list of `<...>` tokens the
skill's templates emit. A token is a placeholder **iff** it is registered — no
shape heuristic — so single-word (`<why>`), spaced and hyphenated tokens are all
handled by construction, and author-written `<...>` prose is never flagged
without a code-span exemption. Fenced code blocks are skipped regardless. Each
entry carries a surface that drives the verdict:

| Token state | Verdict |
|---|---|
| not registered | ignored — the author's own prose |
| registered, in an authored document | WARN; FAIL under `--complete` |
| registered, in a generated document | FAIL always — no human ever fills it |

### 2.14 One EXIT trap, process-wide

`plan-document-lib.sh` installs a single `plan_cleanup` on `EXIT INT TERM` at
load and keeps one accumulating temp list, replacing the per-call
`trap … EXIT` / `trap - EXIT` pair that leaked temps whenever two of them
nested (CODE-STYLE §8).

bash keeps exactly **one** EXIT handler, so the interaction with the ~30
surviving `trap - EXIT` call sites is:

- A script that installs its own `trap … EXIT` *after* sourcing the library
  replaces `plan_cleanup`; its own `trap - EXIT` then clears the slot. Those
  scripts behave exactly as before — their own temp handling is unaffected —
  but they lose `plan_cleanup` for the rest of the run.
- `plan_cleanup` stays installed on `INT` and `TERM` regardless, because
  `trap - EXIT` names only `EXIT`.
- An EXIT handler installed *before* the library was sourced is preserved and
  chained: `plan_cleanup` runs it after itself.

Converting those scripts to `plan_atomic_write`, which is the point of this
vocabulary, removes the overlap. Until then, never use `trap - EXIT` to
"release" a per-call handler.

### 2.15 A working local tree needs the crates built

Run `../setup-dev-env.sh` once after cloning. It builds every crate under
`../src/` for this machine's target triple into ONE `bin/<target triple>` at
the repository root — the directory `plan_bin_dir` walks up to find — rather
than a bin/ inside each skill. Only the host triple is built; cross-building
the other four is what a release (`installer/build-release.sh`) and CI do.

For every planning command that has a `.sh` oracle, the same build also copies
the extensionless Rust executable to `scripts/<command>` beside
`scripts/<command>.sh`. Those sibling files are generated, executable, and
ignored; they are the developer-facing command layout for the Rust migration.
The shared root copy remains because existing shell helpers use it for runtime
discovery, and because the target artifact pipeline still consumes that layout.

Why this needs saying: exactly one artifact is committed, and every helper that
wants a compiled one degrades honestly when it is absent. `plan_crypt_resolve`
falls through to the shell rungs, the chat server drops to an interpreter tier.
So a tree with nothing built still passes the suite, and the compiled path —
the one a target actually runs — is never exercised locally. A green run on an
unbuilt tree is not evidence about the code a user gets.

- **The script refuses to run without nix, and does not fall back to a system
  cargo.** The flake pins the newest stable rust the locked nixpkgs offers,
  with the five house targets; another toolchain produces a different artifact
  from the one CI and a release ship. It exits 69 and prints how to install
  nix.
- **`rjq` is invoked by name.** Building it is not enough — the planning helpers
  find it on PATH, so until the root `bin/<triple>` is on PATH the tree uses
  whatever `rjq` the machine happens to have, or none. The script prints the
  export line; `--check` reports what is present or missing without building.

### 2.16 Generated files are CI's job, not the repo's

- Every binary and compiled output is built by a CI runner and delivered as a
  release artifact. The repo carries no generated files — nothing
  machine-produced is committed, ever. A generated file in git is a blob: it
  cannot be rebuilt on every maintainer box (no darwin or msvc link here), it
  rots out of sight of the build that produces it, and every clone pays for it
  forever.
- This is absolute, and it names the files that are tracked today and must
  leave: the committed `planning/bin/x86_64-unknown-linux-musl/rjq` binary
  (T70a), the compiled `plan-*-lib.sh` outputs of `build-plan-libs.sh` (§2.8),
  `PORTABILITY.md` and `REVIEWER.md`. Each moves to a CI build that publishes
  the artifact, and its in-repo copy is removed in the same coordinated
  change. T73 tracks the migration; its per-file work lands as sub-tasks. (An
  earlier recording of this rule named T71; master had already assigned that
  id to the planning/SKILL.md phase-doc split before it landed.)
- Until a file's migration lands, its existing gate keeps running and a stale
  generated file still fails it. The rule does not downgrade any gate; it adds
  "not tracked" as the required end state. Declared-but-unbuilt is the legal
  resting state on disk (`binaries.tsv` rows; rust-development-guidelines.md
  §6: declare before building — the building happens on the runner).
- Consumers that read a generated file from the working tree — the installer,
  npm pack, test harnesses, `blast-radius.sh`'s freshness checks — must be
  reconciled to fetch the artifact from the release or publish pipeline in the
  same change that untracks the file. An artifact neither tracked nor
  delivered is a broken install, which is the failure this rule exists to
  prevent, so the reconciliation is part of the work, never a follow-up.
- The lint gate is one of those consumers, and not an obvious one. shellcheck
  resolves a `source=` directive only against files named on the same command
  line, so an untracked library drops out of `git ls-files` and every variable
  a sourcing script reads from it reads as unassigned (SC2154). The CI job
  therefore builds the libraries and appends them to the file list; a
  generated file that is linted, or sourced by something linted, belongs in
  that list.
- **`install.sh` is the one standing exception, and it stays committed.** It is
  fetched and run standalone (`curl … | bash`) and is the npm `bin`, so at
  runtime it has no siblings to source and no pipeline to fetch it from — the
  artifact *is* the entry point. It keeps its generator (`installer/build.sh`)
  and its freshness gate (`installer/build.sh --check`, the `installer-build`
  CI job) instead. Read the rule above as "nothing machine-produced is
  committed except the one artifact whose whole purpose is to be downloaded on
  its own", and see `CONTRIBUTING.md` and `RELEASE.md`, which say the same.

## 3. Pending consolidation (the duplication inventory)

The single home for "this logic exists in N places and should be one helper".
It lives here rather than as `# DEDUPE:` comments in the scripts: 58 such
comments existed, 51 of them naming a helper that had already landed, and
`references/comment-discipline-contract.md` clause 5 rules out comments whose
only job is to point at another file. Counts are enforced by
`tests/test-duplication-ratchet.sh`, which fails if any of them grows — that is
what keeps this table from rotting the way the comments did.

| Duplicated logic | Sites | Canonical helper | State |
|---|---|---|---|
| Hand-rolled `"$f.tmp.$$"` + `trap` + `mv` | 43 in 27 files | `plan_atomic_write`, `plan_track_tmp` | helper exists; call sites not migrated |
| Inline `awk -F'\|'` inventory-row parsing with hard-coded field indices (`$2` ID … `$10` Step) | 23 in scripts/*.sh | `plan_inventory_row`, `plan_inventory_rows`, `plan_inventory_split` | helper exists in `scripts/plan-inventory-lib.sh`; the ten work-unit *readers* are migrated. What is left is not all inventory: the two `update-work-unit.sh` rewriters and `plan_prune_work_unit` edit rows in place (a writer helper, not this one), `plan-content.sh find` needs the raw row text rather than trimmed cells, and the rest parse other tables (coverage, `VOICES.md`, the progress trackers). The adversarial-review Findings table is no longer among them: `plan_review_gated_pairs` in `plan-document-lib.sh` owns it, and `mint-fix-keys.sh`, `verify-fix-keys.sh` and `add-fix-claim.sh` all call it — three copies of those field indices were three chances for the writer to accept a pair the verifier does not gate. The cap counts every literal `awk -F'|'` in `scripts/*.sh`, including three docblock mentions and the helper's own parser, so its floor is 1, not 0. Two sites are admitted generic readers rather than migrated inventory parses: `render-plan-overview.sh` cells() (29th) and `remove-coverage.sh`'s outcome match (30th, T17) — a shared canonical-table reader that would absorb both is future work. After the harden-plan-data-parsing goal-03 batches (W09-W13, W26), the shared plan_table_cell/plan_table_set_cell/plan_table_cells helpers also own the plan-content find scanners, both update-work-unit rewriters, the update-step status rewrite, plan_prune_work_unit, the progress status carry, and the propagation/inventory validation readers; the cap fell 30 -> 23 with those batches, then to 21 with W22 (mint-fix-keys) and W25 (cleanup-plans reader), to 19 with W19 (render-plan-overview cells() and the NF probe), and to 15 with W21/W23/W24 (overview-state, both progress counters). The admitted generic readers note is obsolete: cells() is now the shared-helper port. |
| Seed progress-bar literal `` `0%  #### ----------------  100%` `` | 3 files | `plan_progress_bar` | helper exists; glyphs are pinned by `tests/test-progress-bar-shape.sh`, so any migration must stay byte-identical. `rebuild-plan-progress.sh` left the set in T5, which is why the cap is 3, not 4. |
| percent / bar / icon derivation | 3 files | `plan_progress_percent`, `plan_progress_bar`, `plan_progress_icon` | helper exists; `update-progress.sh` is the canonical copy and the library's arithmetic and glyphs are transcribed from it. The other two agree on output but not on spelling, so do not assume a textual match when converting: `update-plan-progress.sh` collapses the filled/empty and icon branches to one-liners. `rebuild-plan-progress.sh` was migrated onto the helpers in T5, which is why the cap dropped from 4. Half-up rounding (`+ total / 2`) and the 20-column default width are part of the byte-identical contract. |
| Status `case` map (`incomplete`/`in-progress`/`completed` → glyph) | 1 file | `plan_status_label` | helper exists in `scripts/plan-document-lib.sh`; `update-step.sh` and `update-plan-progress.sh` are migrated. `rebuild-plan-progress.sh` is the remaining site and is a different shape — it derives the glyph from the goal's own progress file rather than from a requested status word, so it needs a second helper or a rewrite, not this one. The glyphs are the on-disk contract. |
| A test that does not source `lib-test.sh` | 7 of 64 (W17 converted test-progress-helpers to the harness; the remaining seven are benchmark-frozen or self-contained contract probes) | `t_begin` / `t_record` / `t_fail` / `t_end` (`tests/lib-test.sh`) | helper exists; six tests kept a byte-identical copy of its reporter that exited on the first finding, and that count is now 0. The cap counts the library-source line instead, because "a reporter whose body exits" needs brace matching and CODE-STYLE.md section 12 rules out parsing shell structure with a pattern. A `fail() { t_fail "$*"; }` shim is deliberate and is not counted: 32 tests have one, and they are why the call sites did not change. |
| `stat(1)` GNU-vs-BSD probe | `plan-env.sh` + `plan-document-lib.sh` | `plan_stat_mode`, `plan_stat_uid` | helper exists, but `plan-env.sh` sources no library, so migrating it is a structural change |

Migrating any row is a behaviour-preserving change and must be proved as one:
capture the affected scripts' stdout, stderr and exit codes over real inputs
before and after, and diff. `test-progress-bar-shape.sh` and
`test-plan-commands.sh` pin much of the observable output already.

## 4. Change checklist (minimum, per change)

1. Identify every consumer (parser/validator, other helpers, tests, `role_docs()`
   in `role-context.sh`, manifest/map, `install.sh skill_files`, capsule copy,
   hash test).
2. Update shared logic in the library, keep the helper thin.
3. Add/update a regression fixture + test for the new behavior, including the
   actionable-error path.
4. If the change alters a flow that crosses more than two scripts, or adds/removes
   a plan artifact, update the affected diagram in `ARCHITECTURE.md` in the same
   change (`CODE-STYLE.md` §11 picks the diagram form).
5. If a doc changed: keep `SKILL.md` small, update the phase/role docs and their
   references, regenerate `REVIEWER.md` if a reviewer section changed. Keep
   `roles/VOICES.md` registry-aligned and keep `ROLES.md`'s persona doc matrix +
   `plan-context-lib.sh` reader composition in sync with `role_docs()`/`ROLES=()`;
   re-run `test-persona-drift.sh` + `test-voice-artifact-drift.sh`.
6. Register new files in `PACKAGE-MANIFEST.tsv`, `PACKAGE-MAP.tsv`,
   and `install.sh skill_files`. If the file is a benchmark capsule dependency
   (`scripts/*`, `SKILL.md`, `REVIEWER.md`), reflect it in `setup-benchmark.sh`'s
   capsule copy. The manifest line-count is derived from the map (no constant to
   bump) — `test-installer-manifest.sh` asserts the reconcile.
7. Run `bash -n`, `git diff --check`, `test-plan-commands.sh`,
   `test-installer-manifest.sh` (asserts `skill_files()` ↔ manifest/map
   reconcile), `test-plan-env.sh`, the plan validator, and — after any
   role/reader/VOICES change — `test-persona-drift.sh`,
   `test-voice-artifact-drift.sh`, `test-supervision-frame.sh`,
   `test-progress-bar-shape.sh`, and `test-reviewer-projection.sh`. For
   every change under `src/`, also `cargo fmt --check` and `cargo test`
   on each touched crate before pushing: CI runs fmt first and test per
   target leg, so unformatted or failing rust turns four chat legs red
   and burns a cycle.
8. Update the registers, which nothing else will. A defect this change fixes is
   closed in `../BUGS.json` with the commit and the mutation that proves it; a
   defect it *finds* and does not fix is added there rather than left in a commit
   message; queued work goes in `../TODO.json`. Recipes are in
   `../bug-report/SKILL.md` and `../todo/SKILL.md`. This is the one step on this
   list with no gate behind it — no test can tell that a commit resolved a bug —
   so it is the one that gets skipped, and then the next reader has to
   reconstruct the change from its diff.
9. Commit as one coordinated, no-backwards-compat change. The message carries the
   *why* that does not belong in a comment (`CODE-STYLE.md` §12) and names the
   register entries it closes, so the two can be checked against each other.
