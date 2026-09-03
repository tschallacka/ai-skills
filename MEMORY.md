<!-- MODE: DEV -->
# MEMORY — how work goes wrong in this repo

Not a state file. What is queued lives in `TODO.json`, what is broken in
`BUGS.json`, and what is already fixed lives in the git log — this file holds the
diagnostic lessons that are not rules and not defects, and would otherwise be
rediscovered one painful run at a time.

If a line here contradicts the tree, the tree is right: check before trusting it.
An earlier version of this file had two stale entries and one that was wrong from
the start.

## Where the authority actually lives

| Question | Answer |
|---|---|
| How code is shaped | `CODE-STYLE.md` |
| What a script owes everything else | `CODE-CONTRACTS.md` |
| What breaks on BSD or bash 3.2 | `PORTABILITY.md`, generated from `portability-rules.json` |
| What a change must update | `planning/MAINTAINER.md` §4 |
| How a release is cut | `RELEASE.md` |
| Who receives a file | its own `MODE:` / `PACKAGE:` marker (contract 10a) |
| What is queued or broken | `TODO.json`, `BUGS.json` |
| How the diagrams map to the tree | `planning/ARCHITECTURE.md` |

## The traps that keep producing the work

**A check that produces no output is not a passing check.** The single most
frequent source of wasted work here. A grep or `case` pattern that matches
nothing, a reporter called inside `$( )` so its findings go to a subshell, `set
-e` killing a test at an unguarded command substitution. **Run a positive control
before believing a zero.**

**A probe can be pointed where it cannot discriminate.** Worse than a wrong
answer, because it looks like a right one. A mutation dropping the id sort read
green because the rows it inspected happened to sort identically either way; an
allowlist entry looked load-bearing until its removal changed nothing; a
"reproduction" of a long-path failure could not reproduce it because it skipped
the layer that made the path long. Ask what result would *disprove* the probe.

**A cleanup that scans shared state must know what is its own.** Three separate
defects, same shape: `run-tests.sh` deleting another run's test roots, a test's
own `EXIT` trap silently replacing `lib-test.sh`'s, and the verify harness
sweeping a concurrent run's live worktree. The fix each time is an owner marker —
a run id, a pid — never "it matched my pattern, so it is mine".

**An exit code read through a pipe is the pipe's.** `cmd | tail` then `$?` twice
sent an investigation after a defect that did not exist.

**Set-level checks are not content-level checks.** A release list and an `npm
pack` diff both passed a compiled library with a function missing, because both
compared file *sets*. Compare contents when contents are the claim.

**A guard with no test is a claim.** And a guard inside a `while` at the end of a
pipeline is not even that: it runs in a subshell, and on bash 3.2 `set -e` does
not abort on the pipeline's status, so it refuses nothing. When a backstop looks
untestable, build the seam — a stub on `PATH` is usually enough.

**`rjq` given empty input never runs its filter.** It exits 0 having written
nothing, so a writer reports success over a zero-byte file. `rjq -e` also exits
**4** on empty output — but only from 1.7: under 1.6 an empty input still
exits **0**, so a guard keyed on `-e`'s status flips with the rjq version
(B24). Decide emptiness in the script, not in the tool.

**Documented behaviour can still be a defect.** `--skill a --skill b` discarding
`a` was in the README, which made it documented and no less wrong. The manual was
the thing to change.

**A gate that forces a worse artifact is worse than no gate.** The UI prohibition
matched the whole run cache, so a reporter reworded truthful evidence to get past
it. Scope a rule to the field it is about.

**Back up before mutating, never after**, and commit before running anything
destructive. Work has been lost here to `git checkout --` on an uncommitted file.
The complement bit within one session: a mutation applied by sed hit a different
site than intended, the "restore" step was forgotten once, and the weakened rule
slipped into a commit. **A mutation is not closed until `git diff` shows the tree
back at HEAD** — verify the restore with the same suspicion as the mutation.

**Mutation-test every assertion you add.** An assertion never seen to fail is not
verified, and roughly one in five written here was inert until a mutation proved
otherwise. Two refinements from the same day: aim the mutation at the exact line
(a sed matched an identical pattern in another function, and the test correctly
stayed green), and give each refused class its own fixture — a value carrying
several metacharacters hides a dropped rule behind the others that still fire.

## Verified only on this machine

The bash 3.2 floor is real (the flake builds 3.2.57) but **BSD userland is
verified by CI, not locally**. Three defects reached the tree that only the macOS
legs could see: a GNU-only `\|` alternation in `sed` that silently stripped
nothing, BSD `od` padding a trailing space, and an exact path comparison against
`/var`, which is a symlink there. Do not claim BSD behaviour from reading.

**Run one verification at a time.** A wholesale failure with missing-file errors
is a second verification having deleted the first's worktree, not a regression.

## Handoff: planning-helper Rustification (2026-09-02)

The active objective is to finish every item in the separate plan queue
`.plans/planning-rustification/TODO.md`, on the implementation branch below,
then run the full gates, open a PR, and keep the developer-facing wiki notes in
`/home/mdibbets/git/ai-skills-wiki` current.

### Worktree and history

1. Parent repository: `/home/mdibbets/git/ai-skills`.
2. Implementation worktree: `/home/mdibbets/.config/tsch-ai-skills/worktrees/ai-skills-rustify`.
3. Implementation branch: `rustify-planning-helpers`.
4. The `.plans` directory is a separate Git repository. Its queue is on branch
   `master`; update and commit it independently. Do not touch the parent
   repository's `TODO.json` for this initiative.
5. The parent checkout had a pre-existing modified `BUGS.json`; do not carry it
   into the implementation branch or alter it.
6. The latest committed implementation before the current slice was `81be873`
   (`update-work-unit`). Later committed slices include `48bc5df`
   (`update-plan-content`), `0c4e97e`/`27d81fe` (`monitor-read`), and
   `13bccaa`/`882c1b9` (`verify-fix-keys`). The current `overview-state` and
   `plan-content` integration slice is committed; the `plan-context-wrapper`
   slice is now committed separately. The
   separate queue has its own history and must be updated independently.

### Decisions already made

1. Every `planning/scripts/**/*.sh` file is in scope: runtime commands,
   library files, validators, generators, build helpers, and DEV-only tools.
2. Runtime commands become separate extensionless Rust executables with the
   same basename without `.sh`.
3. Shared shell libraries become reusable Rust crates, preserving state and
   invalidation ownership in one crate where they belong.
4. Current shell behavior is the temporary differential oracle. Preserve
   stdout/stderr, exit status, help/error behavior, filesystem effects,
   generated bytes, permissions, and Git snapshots. The old 1.4.2 shell source
   remains recoverable from Git history, not installed permanently.
5. Current `planning/tests/` scripts remain the primary regression suite.
6. Rust commands must be built/tested in the Nix flake development shell, under
   the resource-limited wrapper for substantial commands. The Nix shell emits a
   harmless `cargo186: command not found` warning while still providing cargo.

### Work completed

1. `planning/rust-migration.tsv` inventories all 140 planning shell files and
   classifies each as runtime binary, library crate, absorbed build,
   generator, or generated-retire artifact.
2. Rust crates exist for `planning-core`, `planning-document`,
   `planning-progress`, and `planning-table`.
3. `planning-core` includes safe values, atomic writes, path helpers, and the
   pinned-plan Git snapshot primitive. Snapshot commits use the legacy subject
   `snapshot before <helper>.sh`.
4. Rust binaries added and shell-oracle tested:
   `plan-root`, `update-progress`, `create-progress`, `plan-env`,
   `create-plan-progress`, `create-step-testing`,
   `create-ui-story-run-cache`, `configure-ui-story-cache`,
   `create-ui-validation`, `add-ui-story`, `add-ui-story-links`,
   `update-ui-story`, `create-work-unit-inventory`,
   `create-adversarial-review`, `remove-coverage`, `add-coverage`, and
   `add-work-unit`, `update-plan-progress`, `update-step`, and
   `verify-fix-keys`.
5. Each new crate has a pinned `rust-toolchain.toml`; per-crate generated
   `Cargo.lock` files are intentionally removed before commits.
6. The queue marks the corresponding completed command entries, but most of the
   queue is still unchecked.
7. Wiki pages already added/updated:
   `Planning-helper-rust-architecture.md`, `Planning-helper-parity.md`, and
   `_Sidebar.md`. They describe the layered crates, extensionless binaries,
   parity protocol, and snapshot behavior.

### Immediate next work

1. Continue porting the remaining commands; `overview-state` is implemented
   inside `plan-overview`, and `plan-content` is now implemented as a separate
   read-only binary crate with shell differential probes. `plan-context-wrapper`
   is also implemented and differential-tested; it sources worker variables,
   then delegates to the sibling Rust reader when available. The current seam
   is the cache reader itself.
2. Keep every binary's CLI spelling compatible, including `--plan-dir PATH`
   and `--plan-dir=PATH` where the shell hoist helper accepts both.
3. Add focused Rust tests for non-success paths and shared crate functions;
   compilation with zero unit tests is not sufficient evidence.
4. Finish the reusable inventory/reconciliation/document/validator crates
   before trying to remove shell library files.
5. The root Cargo workspace is now defined and passes workspace formatting,
   check, and test gates; the remaining per-package profile warnings are
   harmless but should be cleaned up when the artifact contract is settled.
6. Integrate binaries into `setup-dev-env.sh`, installer manifests,
   `planning/binaries.tsv`, package maps, benchmarks, and tests only after the
   artifact contract is settled.
7. Replace `build-plan-libs.sh` semantically with Cargo/build artifact logic;
   do not recreate it as a runtime command.
8. Only after all parity and package evidence exists, remove `.sh` oracles,
   run `./run-tests.sh`, full Rust checks, package/installer checks, and create
   the PR.

9. `update-plan-content` covers all documented mutation modes, including
   auto-create/append, CSV newline decoding, review status, decomposition
   review, context invalidation, and step-testing reminders. Direct shell/Rust
   probes match for help, successful modes, malformed CSV, missing paragraphs,
   review approval, and unresolved-review refusal. It is not queue-complete
   until exact error/atomicity probes, focused tests, package integration, and
   full command replacement are complete.

10. `overview-state` is implemented as a second binary in the existing
    `plan-overview` crate. Its extractor now matches the shell oracle exactly
    on the navigation and size fixtures with a pinned `OVERVIEW_NOW`: stdout
    bytes, parsed JSON, stderr, exit status, and help are identical. The parity
    fixes preserve shell quirks such as filepath ordering, trailing section
    whitespace, raw testing-requirement table keys, emoji status markers,
    backtick-only fields, archived finding-cycle detection, malformed
    inventory-row filtering, and the default `generatedAt=serve-live` value.
    `planning/tests/test-overview-state.sh` passes, and `setup-dev-env.sh` now
    builds/copies both `plan-overview` and `overview-state`. The slice is not
    yet committed or marked complete in the separate queue.

11. `plan-context` now has a reusable `plan-context-core` crate and an
    extensionless `plan-context` binary. It implements init/read/check/refresh/
    checkpoint, paging, views, inventory rows, source entries, role aliases,
    UTF-8-safe clipping, snapshot-wide `check --all`, and checkpoint arrays.
    The five available focused shell parity tests pass, as do workspace Rust
    fmt/clippy/test/release gates. The main integration test cannot run in this
    checkout because its required materialized `PLANNING_CONTEXT_CACHE` fixture
    containing W37 is absent; the visible planning-context fixture is only a
    protocol-input directory. Direct checkpoint JSON and role-alias probes also
    match the shell helper.

12. `role-context` now has an extensionless Rust binary with the shell
    registry, aliases, scoped-document output, reviewer pin warning, identity
    gates, path mode, and byte pagination. Differential probes pass for list,
    paths, aliases, access, normal output, small pages, and out-of-range pages;
    its focused Rust tests and capped Nix clippy/test/build pass. The full
    repository suite after the preceding context slice is 122 passed, 0
    failed, 2 expected unconfigured cache-fixture cases.

13. `rebuild-plan-progress` now has an extensionless Rust binary using the
    shared planning-core/progress/table crates. Its progress-derivation and
    plan-dir synonym tests pass against the copied shell-test tree, as do its
    focused Rust test and capped Nix clippy/test/release build.

14. `register-read` now has an extensionless Rust binary using ordered
    `serde_json`, with show/count/report/next-id and a focused filter test. Its
    `list` path delegates the exact current shell filter to the Rust `rjq`
    binary, including the installed parser's current exit-5 diagnostic, so a
    direct stdout/stderr/status probe is byte-identical. The repository's
    `test-register-read.sh` passes.

15. `register-command` now has an extensionless Rust writer using ordered
    `serde_json`, preserving add/remove/list JSON ordering and pretty output,
    validation, and the shell's rjq-unavailable exit-69 contract. Its focused
    unit test, command-registry regression, plan-dir synonym test, and direct
    add/list/remove file/output differential all pass under the capped Nix
    development shell.

16. `register-rebuild` now has an extensionless Rust DEV binary. It stamps
    missing fields, applies the register sort order, preserves mechanical
    writes before semantic refusal, and reports unsound entries. Its focused
    Rust test and capped Nix gates pass; direct shell/Rust fixture comparison
    matches normalized JSON, success output, and damaged-register findings.
    The existing full register-helper test invokes this DEV helper through
    Bash, so it cannot substitute a raw ELF without a test-only shim.

17. The shared `planning-register` library now centralizes ordered JSON
    register I/O, rjq gating, IDs, sorting, timestamps, and soundness findings.
    `todo-add` and `todo-update` are extensionless Rust binaries wired by
    `setup-dev-env.sh`. `todo-add` matches the shell writer on a sound fixture;
    `todo-update` matches status, stdout/stderr bytes, and normalized JSON. The
    direct probe confirmed the shell's duplicate `Updated <id>` line and its
    no-sort behavior, so both are intentionally preserved. Release build and
    focused Nix checks pass. Remaining register work is `bug-add`/`bug-update`
    and broader finding parity before the shared library is considered final.

18. After the OOM-interrupted full suite, focused repository gates exposed two
    integration issues rather than Rust parity defects. `tests/test-mode-markers.sh`
    now classifies Rust `Cargo.toml` files as compiler inputs as documented, and
    the newest crates have the required markers (`MODE: DEV`, `PACKAGE: PROD`;
    toolchain files carry `MODE: DEV` only). `supervision-frame.sh`'s argument
    parser is split into a helper so the existing function-length ratchet stays
    at 58 without changing the shell oracle. Marker, function-length,
    supervision-frame, target-reachability, Bash syntax, and shellcheck gates
    pass; the repair is committed as `d95dd91`.

    The subsequent capped full suite completed in 477 seconds with 121 passes,
    one package-baseline mismatch, and the same two expected unconfigured
    context-cache cases. Updating the owned `npm-package-baseline.tsv` size for
    the helper split made `test-npm-package.sh` pass; no runtime parity test
    failed.

19. `bug-add` and `bug-update` now use the shared register layer and are wired
    as extensionless binaries. Direct shell differentials pass for bug creation,
    note/priority updates, fixed-status evidence, refusal status, output, and
    normalized JSON. `bug-add` deliberately does not sort: the current shell
    helper's implementation contradicts its comment and appends without
    calling `reg_write`; the Rust implementation preserves that behavior.

20. `supervision-frame` is now an extensionless Rust PROD binary wired through
    `setup-dev-env.sh`. It preserves frame field order and byte-budget refusal,
    grant-log tab records, show/check output, and missing-file/status behavior.
    Direct shell/Rust probes for write, check, and grant pass byte-for-byte
    (timestamps normalized only in grant records); capped Nix fmt, clippy,
    tests, and release build pass.

21. `generate-reviewer` is now an extensionless Rust PROD binary using the
    existing `plan-crypt` SHA-256 implementation. It projects the two tagged
    sections, preserves the generated document and missing/empty-section
    refusal behavior, and matches the shell generator's stdout/stderr and
    output bytes in a temporary-skill differential (path names normalized).

22. `cleanup-plans` is now an extensionless Rust PROD binary. It preserves the
    shell helper's direct-child plan discovery, completion-row recognition,
    listing, cancellation, and exit behavior, and delegates confirmed removal
    to the existing Rust `remove-plan` binary. Capped Nix fmt/clippy/tests/
    release build pass; list and declined-confirmation differentials pass.

23. `verify-target` is now an extensionless Rust PROD binary. It parses the
    work-unit inventory, checks target existence/ownership, render-surface
    reachability, layout removal/re-point evidence, and module theme overrides.
    A generated plan/repository fixture matches the shell helper byte-for-byte
    for render and non-render targets; capped Nix fmt/clippy/tests/release build
    pass. Remove/re-point/override branch probes still need a wider differential.

24. `plan-mutate` now has a Rust dispatcher crate and is included in
    `setup-dev-env.sh`. Its help text and unknown-command behavior match the
    shell dispatcher byte-for-byte. It routes only the shell dispatcher’s real
    verbs, maps aliases to the corresponding Rust binaries where available,
    and explicitly falls back to the shell oracle for the still-unported
    inline/validation paths. It passes capped Nix fmt/clippy/test plus direct
    help, unknown-command, Rust-target, and fallback differential probes; this
    is an integration seam, not final shell retirement.

25. The validation library migration has started with `planning-validator-common`
    and `planning-validator-docs`. The common crate centralizes finding output,
    warning/error accounting, Markdown trim, heading checks, and single-field
    extraction. The docs crate ports obsolescence, existence, duplicate step,
    document-heading, review, UI, and decomposition checks. Both pass capped
    Nix fmt/clippy/tests; they are not yet wired into a Rust `validate-plan`
    binary, so shell parity and the remaining validation passes are still open.

26. Added `planning-validator-placeholders` and
    `planning-validator-stale`. The placeholder crate preserves registry-based
    authored/generated findings, fenced Markdown exclusion, and generated goal
    and UI-cache scans. The stale crate preserves paragraph-local history-marker
    exemptions, default/custom phrase selection, companion scanning, and
    advisory warning wording. Both pass capped Nix fmt/clippy/tests; neither is
    wired into `validate-plan` yet.

27. Added `planning-validator-comparisons`, which reads the artifact-comparison
    registry, scans only `## Artifact comparisons` tables in testing companions,
    refuses unknown comparisons and exact comparisons for nondeterministic file
    extensions, and otherwise remains advisory-compatible with the shell pass.
    It passes capped Nix fmt/clippy/tests and is not yet wired into
    `validate-plan`.

28. Added `planning-validator-serve`, which consumes the state-change registry
    and an inventory goal/unit projection, detects state-changing indicators,
    and warns when test/verification acceptance lacks a configured live-serve
    phrase. It passes capped Nix fmt/clippy/tests and is not yet wired into
    `validate-plan`.

29. Added `planning-validator-inventory`, a canonical Rust inventory model that
    parses work-unit rows and coverage ids, preserves declaration/goal order,
    validates row constraints and target paths, checks dependency cycles and
    unknown edges, and exposes transitive dependency queries for later passes.
    Its focused parser/graph tests pass under capped Nix checks; it is not yet
    wired into `validate-plan`.

30. Added `planning-validator-ui`, which parses the nine-column story table,
    validates direct interactions and prohibited shortcuts, checks browser-run
    cache shape/status, related verification units, and bug links. Its focused
    tests pass under capped Nix checks; the renderer is unrelated and remains
    outside this agent's scope.

31. Added `planning-validator-commands`, a reusable Rust port of the command
    literal detector. It preserves fenced/inline span discovery, core and
    registry command candidates, executable/bin path checks, citation/prose/
    directory disqualification, extension registry handling, and advisory vs
    complete-mode findings. Its focused tests and capped Nix fmt/clippy/test
    gates pass. It is consumed by the Rust `validate-plan` orchestrator. The separate
    `render-plans-board` migration remains intentionally excluded because
    another agent owns that renderer.

32. Added `planning-validator-goals`, a reusable Rust port of goal and step
    structure checks: required headings, testing-requirement rows and testing
    bands, goal-size bounds, registered yes/no sections, inventory field
    agreement, atomicity completion checks, and numbered step naming. Its
    focused tests and capped Nix fmt/clippy/test gates pass, and it is consumed
    by the Rust `validate-plan` orchestrator.

33. Added `planning-validator-propagation`, covering completion state,
    verification reachability, testing-companion references, graph leaves, and
    heuristic symbol ownership. It reuses the inventory model and passes
    capped Nix fmt/clippy/test gates. Git freshness and the detailed §9.x
    roster and Git-freshness passes remain orchestration work; the other
    propagation checks are consumed by `validate-plan`.

34. Added the Rust `validate-plan` orchestration binary and wired it into
    `setup-dev-env.sh`. Its CLI/help output matches the shell helper and it
    composes the migrated document, placeholder, stale, inventory, goal,
    UI, serve, command, completion, propagation, and comparison passes. Fresh
    generated-plan differentials exposed and fixed path-ancestor, one-unit
    exception, and failure-gate-summary mismatches; broader mutation coverage
    and final shell retirement remain.

35. Added the Rust `run-adversary-probe` binary and wired it into the dev
    build. It copies the probe fixture including dotfiles, initializes and
    queries the sibling Rust gated reader, checks inventory entries, and emits
    the reviewer prompt. Its focused copy test and capped Nix gates pass; the
    runtime differential passes with the complete sibling-binary build.

36. Built the full host artifact set with `setup-dev-env.sh`. A copied
    `planning-rustification` plan from the separate `.plans` repository now
    validates identically through the shell and Rust `validate-plan` binaries
    in both normal and `--complete` modes (same status/stdout/stderr). The
    shell/Rust `run-adversary-probe` differential also matches byte-for-byte
    after suppressing Rust reader-init output, using absolute script paths,
    preserving escaped JSON, and avoiding inventory coverage diagnostics that
    the probe's shell parser does not emit.

37. The resource-capped full repository suite completed after integration:
    122 passed, 0 failed, with only the same 2 expected
    `PLANNING_CONTEXT_CACHE` cases unconfigured. Workspace-wide Rust fmt,
    clippy (`-D warnings`), and tests also pass.

38. The fresh generated-plan differential now matches Rust and shell exit
    status and stdout; stderr is being rechecked after aligning the exact
    failure-path `Gates:` wording. The fixture uses existing `Cargo.toml` as
    its target and remains outside the repository in `/tmp`.

39. Added propagation roster validation and Git-freshness warnings to the Rust
    crate and wired both into `validate-plan --propagation`. Roster parsing
    follows the shell's §9.1 leading-run and per-unit-blurb rules; freshness
    compares `%cI` timestamps only when the plan is inside the supplied Git
    repository. Focused capped Nix clippy/tests pass. The installed binary
    still needs a rebuild before runtime differential testing of this slice.

40. Resumed after that handoff, rebuilt the complete host artifact set with
    `setup-dev-env.sh`, and verified the copied real
    `/tmp/validate-plan-fixture.sbb1mZ/plan` with `validate-plan --propagation`.
    Shell and Rust have identical exit status, stdout, and stderr there. The
    fresh generated-plan failure differential is also exact after matching the
    shell's missing-review wording and failure-path gate summary.

41. Release/package baseline after the Rust additions is green under the capped
    Nix development shell: `tests/test-release-package.sh`,
    `tests/test-shipped-binaries.sh`, and `tests/test-skill-files-manifest.sh`
    all pass. The release still intentionally packages the shell façade and
    generated `plan-*-lib.sh` artifacts; do not remove or replace those until
    the remaining shell consumers are migrated. `build-plan-libs.sh` is still
    the current shell-oracle build and is the next packaging/retirement seam.

### Pause handoff — resume this evening

1. Worktree: `/home/mdibbets/.config/tsch-ai-skills/worktrees/ai-skills-rustify`.
2. Branch: `rustify-planning-helpers`; clean at pause.
3. Latest commits: `9eddc62` (propagation roster/freshness), `37b4bb6`
   (validator failure parity), `442e3cf` (renderer explicitly excluded).
4. The separate `.plans` repository has pre-existing modified
   `plan-overview-rebuild/context/processed.tsv`; preserve it. Do not hand-edit
   `.plans` plan pages or the parent repository's `TODO.json`.
5. Renderer work remains excluded: another agent owns the Rust replacement for
   `render-plans-board`; leave both that shell helper and its Rust work alone.
6. Wiki files already updated in `/home/mdibbets/git/ai-skills-wiki`:
   `Planning-helper-rust-architecture.md` and `Planning-helper-parity.md`.
7. No long-running process is active. Heavy commands must use the direct
   wrapper `/home/mdibbets/ai/agents/skills/resource-limited-testing/scripts/limited-run.sh`
   and run inside `nix develop .#default`; poll long jobs for roughly 60
   seconds via repeated 30-second waits.
8. Next technical decision: audit and implement the package transition for
   extensionless Rust binaries while preserving the current shell differential
   oracle. First inspect installer/generated-manifest mechanics and the exact
   release tests; do not remove `build-plan-libs.sh` prematurely because shell
   helpers still source its generated libraries.
9. Objective remains unfinished: migrate/retire every in-scope shell helper
   except the externally owned renderer, complete the separate queue's goals,
   update package/install/test/docs contracts, run all gates, create the PR,
   and add/maintain developer wiki entries.

### Verification recipe

`monitor-read` is implemented and wired into `setup-dev-env.sh`. Its Rust
implementation matches the shell for help, show/status/summary/grants/verify,
maintainer gating, missing-frame and budget errors, normal and negative GNU
`tail -n` values, and invalid-number errors. The existing supervision-frame
test passes; packaging and shell-oracle retirement remain global work.

Use commands of this shape from `/tmp/ai-skills-rustify`:

```bash
/home/mdibbets/ai/agents/skills/resource-limited-testing/scripts/limited-run.sh 4G 400 -- \
  nix develop .#default --command bash -lc 'cargo fmt --manifest-path src/<crate>/Cargo.toml --check && cargo clippy --manifest-path src/<crate>/Cargo.toml --all-targets -- -D warnings && cargo test --manifest-path src/<crate>/Cargo.toml'
```

For each helper, build `--release` in that same shell, create parallel shell
and Rust fixtures, compare stdout/stderr, exit status, generated files, and
permissions with `diff`, and compare snapshot commit subjects when the command
mutates a pinned plan. Remove any newly generated untracked `src/*/Cargo.lock`
with `apply_patch` before committing.

`update-plan-progress` has now passed a focused differential fixture, including
both positional and `--plan-dir=` forms. `update-step` has passed the ordinary
shell differential and the matching-Git-diff atomicity fixture. `verify-fix-keys`
has passed a gated shell differential and the existing `test-fix-keys.sh` suite.
`update-work-unit` has passed field, description, and move differentials, plus
the existing `test-plan-commands.sh` regression suite. Its implementation is
committed as `81be873`.
`update-plan-content` has passed direct shell/Rust tree, stdout/stderr, and exit
status differentials across its mutation matrix. `plan-content` has passed
get, summary, find, diff, and blast-radius probes against the shell oracle;
its remaining work is focused edge coverage and final artifact integration.
The resource-capped Nix-shell repository suite completed with 121 passes, one
package-baseline mismatch (fixed and rechecked separately), and 2 expected
unconfigured context tests. Packaging and the remaining helper surface remain
incomplete.

### Compact handoff — 2026-09-02

1. Package staging is committed in `3604a4b`: release preparation now stages
   extensionless Rust planning commands beside their `.sh` oracles, and the
   package/manifest tests cover them.
2. `run-tests.sh` now discovers every `src/*/Cargo.toml`; `planning-map` also
   has the required metadata markers. This gate expansion is committed as
   `ee052ac`.
3. Full capped Nix verification after fixing discovery: `Total ran: 199,
   Passed: 199, Failed: 0, Unconfigured: 2`; the two unconfigured cases are
   the documented `PLANNING_CONTEXT_CACHE` tests. Shellcheck, `bash -n`, and
   `git diff --check` pass for the changed runner.
4. A gated worker found the highest-priority remaining packaging gap: Windows
   planning commands are staged as `.exe` but the generated planning manifest
   requests extensionless names. It also identified missing cross-target
   planning-command CI and that `binaries.tsv` only covers plan-overview/rjq.
5. The `.plans` repository remains separate with unrelated pre-existing
   modifications. Do not hand-edit its plan pages; use planning workers and
   sanctioned helpers. The renderer remains externally owned and excluded.
