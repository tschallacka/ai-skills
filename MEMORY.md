<!-- MODE: DEV -->
# MEMORY — how work goes wrong in this repo

Not a state file. What is queued lives in `TODO.json`, what is broken in
`BUGS.json`, and what is already fixed lives in the git log — this file holds the
diagnostic lessons that are not rules and not defects, and would otherwise be
rediscovered one painful run at a time.

If a line here contradicts the tree, the tree is right: check before trusting it.
An earlier version of this file had two stale entries and one that was wrong from
the start.

## Handoff — ai-text-editor brainstorm (2026-09-02)

- Worktree: `/home/mdibbets/.config/tsch-ai-skills/worktrees/ai-text-editor`
- Branch: `ai-text-editor`
- Source of truth: `.plans/ai-text-editor/brainstorm.md` (gitignored/transient).
- The requested deliverable is a runnable Rust editor server/client plus the
  `ai-text-editor` skill documentation. Do not implement until the remaining
  protocol/safety contracts are resolved or deliberately recorded as defaults.
- Agreed architecture: one Rust server may host multiple file sessions (“editor
  tabs”); one writer per tab; readers may exist and restart after writes. Unix
  sockets are the default on Unix-like systems; Windows uses loopback TCP with
  endpoint discovery and authentication.
- Metadata root: `~/.config/tsch-ai-skills/editor`; use SQLite to map canonical,
  symlink-resolved file identities to journals, sparse indexes, result sets, and
  session metadata. Never derive raw metadata filenames directly from paths.
- Persistence: journal unsaved edits for recovery, explicit save, best-effort
  durability. External changes trigger an alert before the next interaction;
  reload preserves history and records an `external-change` event. If declined,
  offer a `.back` copy; automatic three-way merge is allowed only when
  unambiguous, otherwise return conflict hunks.
- Protocol direction: newline-delimited JSON; server-assigned operation IDs;
  every response includes a document revision; immutable result sets have IDs;
  paging supports opaque tokens and numeric offsets; long jobs support progress,
  polling, and cancellation; edits return all cursor positions. Client session
  lookup follows PTY style: explicit ID, configured agent ID, then environment
  agent ID; stale sessions are reported, not silently resurrected.
- Editing/search: default one cursor, arbitrary numeric cursors, explicit undo
  transactions, Unicode scalar-value coordinates, tabs count as one logical
  column, and large files use lazy configurable indexes plus piece-table/rope
  sparse edits. Search modes are explicit and rich: literal/exact, multiple
  wildcard/glob grammars, multiple regex engines, multiple fuzzy strategies,
  strategy-specific gradients/orderings, line/byte ranges, first-four preview,
  counts, pager IDs, and restart events/delimiters.
- Eve review: the final unrestricted fresh Eve pass read the full brainstorm and
  found no rejected scope, but marked the design **not implementation-ready**.
  Resolve reader subscription/revision semantics, operation result retrieval,
  huge-file completeness/resource limits, binary/encoding rules, journal and
  atomic-save guarantees, merge/conflict semantics, registry locking/identity,
  session leases/authentication, NDJSON/event ordering, and exact help/CLI
  contracts. Full findings are recorded in the brainstorm’s adversarial review.
- Next action: continue the numbered brainstorm questions, update the source of
  truth after each answer, then rerun Eve if the design changes materially. At
  the final gate choose “structured implementation plan” or “implement as is”.

## Where the authority actually lives

## Handoff — ai-text-editor implementation pause (2026-09-02)

1. Implementation is in progress in `/home/mdibbets/.config/tsch-ai-skills/worktrees/ai-text-editor`, branch `ai-text-editor`. The older brainstorm handoff above is historical; implementation has begun.
2. Always run builds/tests through the repository flake:
   `nix develop /home/mdibbets/git/ai-skills --command bash -lc '...'`.
   Heavy commands use the worktree's `resource-limited-testing/scripts/limited-run.sh`.
3. Implemented Rust workspace crates are `src/ai-text-editor` and `src/ai-text-editor-mcp`. The server owns tab state, journals, sparse indexes, SQLite metadata, cursors, revisions, result sets, external-change detection, bounded large-file reads, and Unix/TCP transport. The short-lived client supports structured/text/paging/stream output, session files, searches, edits, cursor actions, recovery, and job commands.
4. Implemented and live-tested: TCP open/edit/undo/read; auth failure and successful auth; session-token save/reuse; journal restart recovery and undo; bounded search and paging identifiers; forced large-file bounded read/index/exact range search; path wildcard; streaming read; job start/complete/poll; installer MCP↔skill switching.
5. The job registry is in `src/ai-text-editor/src/jobs.rs`. It supports queued/running/completed/cancelled/released/evicted states, retention, cancellation race locking, transfer tokens, disconnect behavior, and progress. The server exposes `job_start`, `job_poll`, `job_progress`, `job_complete`, `job_cancel`, `job_transfer`, and `job_release`; the client exposes corresponding hyphenated commands; MCP advertises the protocol methods.
6. Installer switching is in `installer/src/05-config.sh`, `10-cli-args.sh`, `50-manifest.sh`, `55-cli-handlers.sh`, and `60-install.sh`, then regenerated into `install.sh` with `./installer/build.sh`. Use `--editor-integration skill|mcp`; default is `skill`. Cleanup is allowlisted to editor binaries and does not stop servers or touch sessions/metadata. A real temporary-root switch test passed.
7. Generated schemas live in `ai-text-editor/schemas/`; platform release binaries are CI artifacts and are not committed. The artifact workflow is declared at `.github/workflows/ai-text-editor-artifacts.yml` for musl Linux, macOS, and Windows targets. Do not claim those binaries exist locally unless a target build has actually produced them.
8. Current clean checks: `cargo clippy --workspace --all-targets -- -D warnings` passes; `cargo test --workspace` passes with 18 tests; `installer/build.sh --check`, `bash -n`, shellcheck, and `git diff --check` pass. The full `./run-tests.sh` was started under a 2G/300 resource cap but is not green yet.
9. The latest full-suite failures requiring repair are structural packaging issues: `test-mode-markers` sees the new `src/ai-text-editor*` files without `MODE:` markers; `test-release-package` discovers an unrelated existing `src/chat-server-rs/Cargo.toml` workspace issue; `test-skill-files-manifest` reports missing chat artifacts and does not account for `ai-text-editor/agents/openai.yaml` or the generated schemas. Resolve these directly using repository helpers/conventions before rerunning the full suite.
10. Do not dispatch new reviewers. User explicitly requested self-audit, then implementation. Continue with the failing suite repairs and remaining plan work; do not mark the goal complete yet. Use `apply_patch` for source edits and never hand-edit generated `install.sh`.
11. Since this pause, the structural suite repairs are green: editor source markers, workspace exclusions, installer manifest registration, host chat artifacts, CLI copy-loop extraction, generated npm baseline, and persona/package checks. The full suite reached 117 passed, 5 failed, 2 unconfigured before those repairs; rerun it after the next implementation slice.
12. Added explicit CLI help for all commands/modes/responsibilities and MCP `resources/list`/`resources/read` for the protocol schema, capabilities schema, and man page. Canonicalized existing file identities for endpoint, SQLite, and journal keys and corrected the fallback root to `~/.config/tsch-ai-skills/editor`. Rust gates pass again; generated Linux binaries and installer are refreshed.
13. Remaining known issue from the prior full-suite run: `test-runtime-dependencies` still reports the pre-existing `register-command.sh` rjq-less probe mismatch under the full fixture; investigate with an isolated positive/negative reproduction before changing it. Remaining product gaps include true large-file edit jobs, journal append failure atomicity, richer external-change/reader restart semantics, and complete MCP/session lifecycle behavior.
14. The whole suite subsequently passed 122/122 (2 optional context tests unconfigured). Added real `large_edit`: requires `job_id`, current revision, and `acknowledge_large_edit=true`; streams a bounded replacement through a same-directory temp file, syncs and renames, refreshes the sparse index, updates metadata, and completes/fails the job. Added a live release-binary test proving the rewrite.
15. MCP now exposes local schema/man-page resources and explicit tool descriptions. Ordinary edits are journal-first; external reload/merge journal records now include before/after snapshots so restart recovery can rebuild undo history. Every Rust gate still passes after these changes; run the whole suite again after the next non-Rust packaging change.
16. Added `history` read-only dispatch/client/MCP operation returning undo/redo depths and journal sequence. Mutation responses now include the cursor map; text/raw/hex cursor offsets are adjusted across ordinary byte-range edits. Updated the man page and protocol docs for large edits/history. The full suite was green before this latest Rust-only change; current Rust gates remain green.
17. Latest Rust hardening adds structured external-change error details (byte count, allowed choices, and the force-save acknowledgement requirement), private permissions for the session-token file and editor metadata root, and protocol documentation for those fields. These changes were made with `apply_patch`; do not hand-edit generated installer output.
18. The latest source changes also add cursor maps to undo/redo and large-edit mutation responses, and add a history-depth operation to the client/server/MCP surfaces. The revision guard still needs inspection to ensure `large_edit` is included; this is a known follow-up before claiming the mutation contract is complete.
19. A resource-limited Nix verification command was launched after the latest changes (`cargo fmt`, clippy, workspace tests, release build, schema generation, installer regeneration, diff check), but its output was truncated by context compaction. Inspect the process/state before rerunning so duplicate builds are not started. The prior Rust gates passed; the whole repository suite must be rerun after the current slice.
20. Product gaps still requiring implementation or an explicit documented contract: real reader pause/restart delimiters during writes; close/tab lifecycle with an explicit preserve-or-clean journal decision; session leases and multi-tab isolation; transactional journal handling for every mutation, including large edits; large-edit undo/recovery; richer fuzzy-gradient semantics; normalization/restoration correctness; raw/hex editing completeness; durable SQLite result-set persistence; and complete Windows atomic-replace behavior. No new reviewers are wanted; resolve and self-audit these gaps.
21. The repository's `planning/tests/lib-test.sh` documents the Unix socket path ceiling and deliberately uses short test roots. During a live editor probe, two separate application bugs were found and fixed: the socket key is now shortened to 32 hex characters, the socket parent is created before bind, and discovery metadata is separated from the bound `.sock` path (`.endpoint` remains the client lookup file). This was verified with a custom long `XDG_RUNTIME_DIR` and the close lifecycle.
22. Close lifecycle is now implemented: `close` without `journal_action` returns a structured `journal_close_decision_required`; `preserve` journals and terminates; `clean` removes journal/SQLite artifacts and terminates. Client, protocol reference, man page, MCP tool list, and generated capability schema source are updated. Release build and schema generation passed after the change.
23. The skill-creator `quick_validate.py` was invoked, first directly (permission denied because the helper is not executable) and then through Python; the latter is blocked only because the ambient Python lacks PyYAML. Do not treat that as a skill-content failure; rerun through an environment containing `yaml` when available.
24. The full suite after the close/socket/session work reached 121/122 passed, with only `test-npm-package` failing from expected size drift. The baseline was regenerated from `npm pack` using the Nix flake (capturing only the final package filename), and the targeted package test now passes. A prior capture attempt was invalid because the flake banner was included; always strip that banner before reading the package path.
25. The socket discovery probe exposed and fixed the distinction between the bound Unix socket and endpoint metadata. `endpoint_for_file` is the `.endpoint` discovery file; `socket_for_file` is the `.sock` listener path. The server creates the runtime parent before binding. The live test used an intentionally long runtime directory and passed.
26. Session behavior now follows the inspected interactive-shell contract: explicit endpoint/token, then named session, named agent, and `TSCH_AI_EDITOR_AGENT`, `CODEX_AGENT_ID`, `AGENT_ID`; named sessions can be initialized with `--file` and are automatically persisted under the metadata/session directory. The refreshed release binary was live-tested through open, history, and close.
27. Fuzzy search now accepts `gradient` in the inclusive range 0.0..1.0, with strategy-specific defaults/meaning, and has a unit test. The 20-test Nix Rust gate passes after this change.
28. The subsequent full Nix/resource repository suite is green: 122 passed, 0 failed, 2 optional `PLANNING_CONTEXT_CACHE` tests unconfigured. Release artifacts, schemas, installer, and npm baseline were refreshed before that run.
29. Added explicit `begin_transaction`/`end_transaction` protocol, client, MCP, schema, man, and protocol support. Ordinary edits inside a transaction remain individually journaled but are grouped into one history undo record. The 20-test Nix Rust gate passes, and a refreshed release live probe confirmed two inserts yield one undo step and restore correctly.
30. `skill-creator` validation passes when Python is given the Nix PyYAML store's `lib/python3.14/site-packages` through `PYTHONPATH`; direct ambient Python and a plain `nix shell` did not expose the module. Use the working store-path method rather than modifying the helper or the skill.
31. Close shutdown now removes the `.endpoint` discovery record and owned `.sock` path before exiting. The Rust gate passed after this change, and all release artifacts were rebuilt. The skill text was corrected to document the actual session flags/environment order.
32. The final whole-repository Nix/resource suite for this slice is green: 122 passed, 0 failed, 2 documented optional context tests unconfigured. It includes packaging, installer, mode-marker, shell portability, schema, and cargo-plan checks; it does not prove all editor runtime contracts.
33. Added explicit `--normalize-nfc` server startup selection and exposed `normalize_nfc` from `open`. `Document::coordinate` addresses stored bytes, while normalized search uses presentation offsets safely. A live decomposed-to-composed exact-search probe passed; protocol and man documentation were updated.
34. After transaction, normalization, and metadata changes, release artifacts and generated schemas/installer/npm baseline were refreshed. The next required check is the full Nix/resource repository suite; do not claim final completion until remaining product gaps in entries 20 and 32 are audited.
35. The final suite after this slice completed green again: 122 passed, 0 failed, 2 documented optional `PLANNING_CONTEXT_CACHE` tests unconfigured. The man page was corrected to real roff escapes (no control-byte form feeds), and the package baseline was regenerated from Nix `npm pack` afterward.
36. The user has now explicitly paused the work and requested this memory handoff. The current implementation slice added `save_as`: it is allowed through a pending external-change alert, requires a new `target_path`, refuses an existing target, atomically writes the current view to the new path, journals the operation, and returns the new active path/revision. Client, MCP, schema source, protocol reference, and man page were updated with `save-as`/`save_as`. This slice still needs Nix Rust verification, release/schema/installer regeneration, package-baseline refresh, and the full repository suite before its status can be trusted.
37. The last command was a resource-limited Nix run of `cargo fmt --all`, clippy, and workspace tests. Its output was truncated by context compaction, so inspect for an active `cargo`/wrapper process before starting a duplicate. If it is finished, rerun the gates and then the required artifact/package/full-suite checks one at a time through the flake and resource wrapper.
38. The user’s latest concern is that the repository maintainer/developer documentation records known limitations. Treat those documents as authoritative context: consult the already-ingested `AGENTS.md`, `DEVELOPMENT.md`, `CODE-STYLE.md`, `CODE-CONTRACTS.md`, generated `PORTABILITY.md`, and relevant planning maintainer references before making future structural changes. Do not infer that a green repository suite proves the editor product is complete.
39. Preserve the explicit unresolved product audit from entries 20 and 34: one process currently owns one file/tab rather than a multi-tab server; SQLite persists summaries but not reusable exact indexes/results; large-edit undo/recovery and fully transactional journal failure handling are incomplete; external reader restart/delimiter behavior is only contract-level; normalization/raw/hex restoration remains partial; authentication leases/revocation and Windows atomic replacement need review; fuzzy gradients are exposed but simplistic. Resolve or document each before claiming completion.
40. Added exact SQLite persistence for `line_index_block` rows and `result_match` rows, with per-tab metadata still server-only and WAL/NORMAL/temp-file settings intact. Initial and refreshed indexes now persist their blocks; searches persist the full result rows before putting them in the in-memory pager. The Nix Rust gate passes.
41. Raw-byte/hex cursor actions are now mode-aware, including home/end, byte/page movement, explicit coordinate clamping, and implicit `--cursor-id` offsets for insert/replace when `--offset` is omitted. TCP startup now requires `--auth-token`; session parent directories are private. External disk identity now tracks size, mtime, permissions, and content digest for normal files.
42. External reload and merge now append their before/after journal snapshots before committing document/history state. Large-file edits now stream file-backed before/after snapshots, journal their sidecar paths before acknowledging the mutation, expose large undo/redo, reconstruct available large history at startup, and remove those snapshots on `close --journal-action clean`. Nix clippy/tests pass and release artifacts were rebuilt.
43. The full repository suite after the metadata slice had 121 passed, 1 failed (`test-install-ui`, exit 28) and 2 expected unconfigured context tests; the same UI test reproduced directly under the flake passed. Treat the full-run failure as transient until a subsequent one-at-a-time run confirms it. Package test passed before the latest docs/source changes; regenerate the npm baseline and rerun it after this slice.
44. Added a mode-aware raw/hex cursor implementation and implicit `cursor_id` offsets for insert/replace. Added sequence and canonical-payload byte counts to response frames, a standalone `backup` external-change action that leaves resolution pending, authenticated-TCP startup enforcement, metadata/permission identity tracking, and journal-first reload/merge commits.
45. Added durable large-file before/after sidecar snapshots, atomic large undo/redo, journal replay reconstruction where sidecars survive, and cleanup on `close --journal-action clean`. Added explicit lossless NFC `restore` operation with conflict refusal, and an end-to-end `tests/test-ai-text-editor.sh` runtime test. The long-path runtime integration test passes.
46. Fixed Unix socket discovery for long temporary runtime paths by falling back to a short `/tmp/tsch-ai-skills-editor` root when the configured path approaches `SUN_LEN`; the integration test specifically exercises this through the resource wrapper.
47. The final Nix/resource whole-repository suite after these changes is green: 123 passed, 0 failed, 2 documented optional `PLANNING_CONTEXT_CACHE` tests unconfigured. Rust clippy/tests (21 unit tests), release build, generated schemas, generated installer, npm package baseline/test, skill-creator validator, shellcheck/bash syntax, `git diff --check`, and the runtime integration test all pass.
48. Final follow-up hardening: the integration test initially exposed the `pipefail-grep-q` portability rule and was corrected to use a `case`-based `contains` helper; the subsequent full suite is green at 123/123. Added a standalone `backup` external resolution action, protocol sequence/byte-count fields, explicit lossless NFC `restore`, mode-aware raw/hex cursors, implicit cursor-based edits, authenticated TCP refusal without a token, and long-runtime socket fallback. Keep generated binaries/schema/installer/package baseline synchronized after any further source change.

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

## 49. PR 17 compatibility audit (2026-09-03)

PR 17 is open as `planning: rustify helper tooling` (`a53d12d`). It changes the
repository-wide Rust build shape: the root workspace becomes `members =
["src/*"]`, build output is centralized under `/target`, and the installer,
release builder, CI, and package baselines gain Rust-artifact handling. It does
not contain the editor crates.

The editor branch must therefore remain additive. Its root `Cargo.toml` now
uses the same `src/*` workspace glob, retains the editor workspace package and
dependency declarations, and includes the PR's root `profile.dist`; its lockfile
was regenerated for both the existing planning crates and the editor crates.
`setup-dev-env.sh` now reads central Cargo output and lists the editor binaries.

The exact PR ref is available locally as `origin/pr/17`. Shared files that will
need a careful three-way merge are `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`,
`setup-dev-env.sh`, `.gitignore`, `install.sh`, the installer fragments,
`package.json`, and the package baseline. Do not blindly take either side: retain
the editor manifest rows, platform artifacts, MCP/skill swap logic, and editor
runtime dependencies while taking PR 17's central-target and generated-manifest
rules.

PR-owned prerequisite found during the audit: with the current PR ref and Rust
1.98, `cargo fmt --all --check` reports formatting differences in
`src/plan-overview/src/main.rs`. This is not an editor change and was not altered
on this branch; it must be fixed in PR 17 or resolved when it lands before using
the full workspace format gate as evidence.

The compatibility changes were validated with the combined workspace lockfile,
editor-only fmt/clippy/tests (21 Rust tests), the editor integration test, the
installer manifest test, shell syntax, whitespace, and `./run-tests.sh` under the
Nix flake. All local checks passed; the full-workspace format check remains
blocked only by the PR-owned `plan-overview` formatting finding above.

## 50. Persisted index reuse (2026-09-03)

Closed a real implementation gap in `metadata.rs` and the server: startup now
loads a persisted line index only when granularity, byte length, disk identity,
revision, completeness, ordered blocks, and block bounds all validate. Missing
or stale/corrupt rows trigger a rebuild; large tabs use their bounded file index
builder. The `open` response exposes `index_loaded` for machine-verifiable
diagnostics. Closing with `journal_action=clean` now removes the journal and
large-history sidecars but retains the per-tab SQLite cache, so a later tab
startup can reuse indexes/results. The integration test proves a restart cache
hit. Editor clippy/tests and the shipped-binary end-to-end test pass after the
change.

## 51. Paused after persistent result paging (2026-09-03)

The user requested a pause immediately after the next persistence improvement.
`metadata.rs` now also has `load_result_matches(result_id, revision)`: it loads
only complete result sets for the current revision, validates the persisted row
count and JSON, and treats missing/partial/corrupt data as a cache miss. Server
`page` now attempts that reload before returning `stale_result`, and caches the
validated rows in memory. A metadata unit test covers a successful load and a
revision mismatch.

The result-paging change was then compiled and passed editor fmt/clippy and all
21 Rust tests. The shipped Linux binaries were rebuilt, and the end-to-end test
now proves search → save → preserve journal → restart → page by the old pager
key; it passes. The sandbox initially refused the temporary Unix socket, so the
integration run was repeated with the approved socket-capable execution path.

The broader implementation remains active. PR 17 is still open; the PR-owned
`src/plan-overview/src/main.rs` formatting finding and the plan's stale 0%
progress remain documented in entries 49–50. The next major unimplemented
contract gaps are authenticated HMAC challenge/session lifecycle, fuller large
file external-resolution jobs, and the plan-owned verification targets.

## 52. Paused after authenticated TCP wiring (2026-09-03)

The user paused implementation and requested this memory handoff. The editor
auth module and TCP challenge/proof path are now wired and compile: each TCP
connection receives a nonce and server generation, the client proves knowledge
of the configured secret with HMAC-SHA256 over a length-prefixed transcript,
and the server then processes the original request through the existing
authenticated request path. Empty TCP auth tokens are refused at startup;
Unix transport remains unchanged. Auth unit tests cover transcript stability,
exact proof verification, mismatch refusal, nonce generation, and decoding.

The Nix editor gate passed after this wiring: editor fmt, clippy with denied
warnings, and 24 Rust tests. Release binaries are CI artifacts and are not
committed. No TCP runtime integration test
has yet been run after the handshake wiring. The next concrete check is to
start the shipped server on `tcp:127.0.0.1:0`, parse its announced endpoint,
verify a correct token can open, and verify an incorrect token is refused
without mutation. Socket-capable execution may require the approved elevated
path.

Known follow-up remains: the current handshake is not yet the complete planned
credential/session lifecycle. It still needs review or implementation for
credential-file ownership, expiry/skew, replay protection, rotation and
revocation, and generation lifecycle. The protocol schema, skill help, man
page, and protocol reference should also be audited so they describe the
challenge/proof exchange rather than implying a plaintext TCP token. Do not
claim the overall editor objective complete; the implementation plan still
contains unfinished product contracts and progress metadata must be updated
through planning helpers only.

PR 17 remains open and must not be merged blindly. Its central Cargo target,
workspace/installer/release changes still require a three-way merge that keeps
the editor dependencies, artifacts, MCP/skill swapping, and auth implementation.
The PR-owned `src/plan-overview/src/main.rs` format finding remains separate.

## 53. Large-range, historical-page, and cleanup hardening (2026-09-03)

After resuming, the TCP authentication path was hardened against proof replay:
the server now requires the client-provided nonce to equal the fresh nonce it
issued, and the TCP client strips `auth_token` from the editor request after
using it locally for HMAC proof. Protocol reference, skill, docs README, and
man page now describe the challenge/proof exchange accurately. The shipped
TCP test proves good-token open/read and bad-token refusal.

Large tabs no longer silently perform unbounded exact-text scans. They require
an explicit inclusive line range and report `search_range` coverage. Large
exact-byte searches now accept an explicit bounded half-open byte range up to
the read ceiling and return absolute byte offsets plus base64 contents. Client,
protocol, man, and skill help expose those flags; the runtime test covers both
large search modes.

Paging now supports an explicit historical read (`--historical`) of a complete
persisted result from an older revision, returning `source_revision` and
`stale: true`; default stale paging remains rejected. Metadata has a validated
historical loader and a unit test.

Explicit `close --journal-action clean` now closes the file-backed SQLite
connection before removing the per-tab database/WAL/SHM files. The integration
test waits for each short-lived server process, closes the TCP tab explicitly,
and verifies no tab database remains after cleanup. A direct metadata cleanup
unit test also passes. Release Linux binaries were rebuilt after the changes.

The full repository suite was green immediately before this slice (123 passed,
2 optional context tests unconfigured); after this slice, rerun the full suite
and refresh the npm baseline after the final documentation edits. Remaining
major plan gaps still include multi-tab server hosting, full endpoint/session
lease and credential rotation lifecycle, live reader restart semantics,
complete index/search engine coverage for huge files, and final packaging/plan
verification.

## 54. Credential-file rotation and historical search contract (2026-09-03)

The server now accepts exactly one of `--auth-token TOKEN` and
`--auth-token-file PATH`. On Unix, the credential file must not be
group/world-readable; it is read at startup and reread for each TCP connection,
so replacing the file rotates the accepted secret without restarting the
server. The client never serializes its token in the post-handshake editor
request. The shipped integration test covers private-file startup, old-token
rejection after rotation, new-token success, and authenticated clean close.

Large-file exact text search now refuses an omitted end line; exact-byte search
accepts a bounded half-open byte range and returns absolute offsets/base64
contents. Search responses include requested line/byte coverage. The client,
skill, protocol reference, docs README, man page, and client help were updated.
Explicit historical paging (`--historical`) loads a validated complete result
from SQLite after a revision change and reports its source revision/stale state;
default paging still refuses stale results.

The per-tab SQLite cleanup path now closes the file-backed connection via an
in-memory replacement before removing the database/WAL/SHM files. The runtime
test waits for each short-lived server process and verifies clean close leaves
no tab database. Current editor verification is green: 25 Rust tests,
release binaries, and the editor integration flow. The whole repository suite
must still be rerun after the latest client-help/docs/test changes; regenerate
the npm baseline after those edits. Do not claim overall completion: multi-tab
hosting, separate session authorization/leases/revocation, live reader
restart semantics, and full plan-owned verification remain open.

## 55. Updated from merged master (2026-09-03)

The branch fast-forwarded to `origin/master` at merge commit `e9025d5`, which
includes PR #17's Rust workspace migration and centralized root `target/`
artifacts. The editor workspace dependencies were retained and `setup-dev-env.sh`
now emits the editor and MCP binaries alongside the planning tools. The installer
was regenerated with `./installer/build.sh`; generated package metadata was
refreshed after explicitly indexing the editor files.

Post-merge verification: editor format, clippy, and 25 unit tests passed; isolated
editor integration, package, chat-resolution, and chat tests passed; the complete
repository run reached 199/202 passed with only the two expected unconfigured
context tests, while its three chat/package failures were caused by concurrent
test runners and the pre-indexed package baseline. Those three checks passed when
rerun alone. The implementation remains intentionally uncommitted and the
pre-merge stash checkpoint is retained for recovery.
