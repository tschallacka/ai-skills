# Reviewer optimization action plan

## Objective

Reduce benchmark token usage by retaining reviewer context across correction
passes, while preserving an independent final quality check and comparable
benchmark results.

## Proposed review lifecycle

1. Start Reviewer A in a fresh session for the initial adversarial review.
2. Reviewer A records stable finding IDs and enters a bounded fix-verification
   state.
3. The planning agent fixes the findings. Reviewer A receives only the changed
   files, relevant diff, and targeted validation output.
4. Reviewer A may approve closure of its own findings, but may not approve the
   overall plan or redefine the review contract.
5. When all of Reviewer A's findings are closed, Reviewer A writes a concise
   handoff and terminates immediately—the reviewer effectively completes its
   task and dies.
6. Start Reviewer B in a fresh session. Reviewer B performs a fresh full review
   of the resulting plan and does not inherit Reviewer A's conversation.
7. If Reviewer B finds issues, repeat the same fix/verify/terminate cycle with
   Reviewer B, then spawn Reviewer C or the next fresh reviewer.
8. Record every reviewer session, cycle, finding owner, closure pass, and final
   independent approval.

## Contamination controls

- A reviewer may verify only findings it originally created, identified by
  stable `AR-NN` IDs.
- A reviewer may not mark the entire plan approved, even when all its own
  findings are closed.
- A fresh reviewer must inspect the final plan independently and may reopen
  old issues or add new findings.
- A review is not considered independent if it reuses a prior reviewer's
  session or receives that reviewer's conclusions as authoritative context.
- If a reviewer cannot close its findings within the pass limit, terminate it
  and start a fresh reviewer; do not extend one context indefinitely.
- If a fix changes the task contract, required artifacts, or safety boundary,
  require a fresh review immediately.
- Enforce a maximum total review-cycle count and fail or mark the benchmark
  unresolved when the limit is reached.

## Token-control measures

- Use one batched patch for related artifact changes.
- Pass diffs and targeted file excerpts instead of rereading the whole plan on
  every verification pass.
- Limit reviewer responses to findings, affected files, required corrections,
  verification result, and unresolved risks.
- Avoid repeated full-directory listings and repeated full report dumps.
- Run the complete validator once after the correction cycle, before Reviewer B.

## Required protocol changes

- Add an explicit iterative-review mode, separate from the default fresh-review
  mode.
- Define the maximum verification passes per reviewer and maximum fresh-review
  cycles per benchmark.
- Add reviewer lifecycle fields to the evaluation/report data:
  `review_cycle`, `reviewer_session`, `finding_owner`, `verification_pass`,
  `closed_findings`, `reviewer_handoff`, and `review_mode`.
- Require the analyzer to report fresh-review cycles, verification passes, and
  reviewer termination/handoff events separately.
- Mark review independence as compromised if a fresh reviewer inherits a prior
  reviewer's conclusions or session.
- Preserve the current fresh-review behavior as the default for cross-version
  comparisons.

## Validation plan

Before enabling this in the main benchmark:

- Run a small pilot on one or two revisions.
- Compare total tokens, reviewer event count, review findings, fix count,
  final validation status, and taint rate against fresh-review mode.
- Confirm Reviewer B catches issues that Reviewer A could have overlooked.
- Confirm the result archive records the complete reviewer lifecycle.
- Adopt iterative mode only if it reduces tokens without reducing independent
  defect detection.

## Decision rule

Use iterative review when token efficiency is the priority and the final fresh
review remains mandatory. Use fresh-review mode for strict historical or
scientific comparisons where reviewer independence must be maximized.

## Reviewer profile generation

The reviewer profile is a new 1.4.2 contract. Older skill versions do not use
`REVIEWER.md`, and no backward-compatibility path is required or permitted.

- `SKILL.md` remains authoritative for the new version.
- Reviewer-specific source sections are marked with `REVIEWER_SECTION` markers.
- `scripts/generate-reviewer.sh` has an explicit section allowlist and fails if
  a required marked section is missing or empty.
- After any reviewer-relevant `SKILL.md` change, an AI agent must run the
  generator and commit the resulting `REVIEWER.md` update.
- `REVIEWER.md` records the source hash and is not hand-edited.

## Hard access controls for 1.4.2

Prompt wording is guidance, not an enforcement boundary. The 1.4.2 benchmark
must use filesystem scoping in addition to the allowlist instructions.

- Build a clean per-worker input capsule containing only the task specification,
  the tagged `planning/SKILL.md`, `REVIEWER.md` for reviewers, and explicitly
  resolved relative references.
- Do not expose the complete tagged source checkout through `--add-dir`; the
  worker must receive the capsule instead.
- Keep the capsule, worker workspace, and result archive as separate roots.
- Create a fresh capsule for every fresh reviewer so no previous reviewer
  conversation or unapproved files are inherited.
- Keep harness-only validator scripts outside the agent-readable capsule when
  the worker does not need to inspect them directly.
- Audit worker command paths and filesystem access after completion. Attempts
  to inspect repository history, installed skills, previous results, parent
  directories, or unallowlisted source files taint the run.
- Treat broad commands such as unrestricted `find`, repository-wide `rg`, or
  `git log` as violations when they target outside the capsule/workspace.
- Retain the explicit prompt allowlist and refusal instructions as a second
  layer, but do not rely on them as the primary control.

## Context variables and bounded-read integration

- Generate a per-worker `.bm-vars` file with the current capsule, workspace,
  plan, skill, and helper paths.
- Do not use one shared `/tmp/.bm-vars`; parallel workers must not be able to
  overwrite or source one another's variables.
- Add a short wrapper that sources the per-worker variables and invokes the
  existing `plan-context.sh` reader with the correct plan directory.
- Keep the existing bounded text output and views; do not replace them with
  verbose metadata or full-file dumps.
- Initialize the context snapshot once, use `check --changed` before rereading,
  and read only targeted views or changed entries.
- Extend the context index with a source namespace for `SKILL.md`,
  `REVIEWER.md`, and approved relative references, using the existing hash
  freshness checks.
- Keep memory state outside counted plan deliverables and invalidate it when a
  source or plan file changes.
- Continue using the existing guarded planning shell helpers for writes; batch
  related writes through those helpers instead of emitting raw patch blocks.

## Additional token optimizations

- Add phase checkpoints after drafting, review, correction, and validation.
  Each checkpoint should contain only the current state, open findings, next
  action, changed files, and relevant hashes.
- Permit a fresh phase context to start from the checkpoint when carrying the
  accumulated conversation would cost more than reloading the compact state.
- Add size budgets for analysis reports, review reports, progress files,
  testing companions, and context summaries. Fail or warn on unnecessary
  prose while preserving required evidence.
- Make planning helpers quiet by default. Return concise status, changed path,
  validation state, and next action; require an explicit verbose option for
  full output.
- Use phase-specific read views: summaries during drafting, ownership and
  dependency views during review, changed documents during fixes, and focused
  validator output during final validation.
- Avoid duplicating the same task details across companions and reports;
  reference authoritative artifacts when repetition is not required by the
  schema.
- Add bounded retry behavior for malformed helper calls. Return the corrected
  usage shape without causing a full-plan reread.
- Measure each optimization with command count, command/output size, context
  reads, review events, retries, artifact sizes, and total telemetry tokens.

## Latency optimizations

- Measure elapsed time by phase: setup/capsule creation, initial reading,
  drafting, review, fix verification, fresh-review cycles, validation, and
  final reporting.
- Pre-build the per-worker capsule, `.bm-vars`, context index, and helper
  wrappers before starting the agent so startup work is not paid in the agent
  conversation.
- Generate canonical plan scaffolding and required artifact skeletons with the
  guarded shell helpers before the agent begins semantic planning.
- Perform one coherent draft pass before review instead of reviewing partial
  artifacts repeatedly.
- Bound both review passes and wall-clock time per phase. On timeout, write a
  checkpoint and stop cleanly rather than continuing an unbounded loop.
- Allow independent read-only analysis tasks to run concurrently, but keep
  plan mutations serialized through the guarded helpers.
- Run validation only after a complete correction batch unless a targeted
  check is needed to diagnose a specific failure.
- Stop once the plan, required artifacts, independent review, and validation
  gates are complete; do not spend additional turns polishing non-required
  prose.
- Consider a faster reviewer/formatter profile for deterministic artifact
  maintenance, while keeping the substantive final reviewer independent.
- Report latency and quality together so a faster run is not accepted if it
  increases taint rate, missed findings, or invalid artifacts.

## Access-control validation

The 1.4.2 pilot must prove that:

- allowed files remain readable;
- an unallowlisted source file is unavailable to the agent;
- previous result directories and installed skills are unavailable;
- the command/path audit detects attempted escapes;
- a detected escape produces a tainted evaluation and an explanatory report;
- fresh reviewer capsules remain independent across review cycles.

## Benchmark protocol transition

Backward compatibility is explicitly not required.

- Treat all results through 1.4.1 as the frozen `legacy` benchmark cohort.
- Do not rerun or retrofit the legacy cohort with the new reviewer, capsule,
  context-variable, or compact-read behavior.
- Start the new protocol at 1.4.2 with a distinct protocol identifier such as
  `benchmark-protocol-1.4.2`.
- Record the protocol identifier, reviewer mode, and access-control mode in
  every new result archive and evaluation file.
- Keep the existing result directory layout and historical artifacts unchanged.
- Make the final analyzer discover and merge legacy and new-cohort results into
  one user-facing report.
- The merged report must separate cohort summaries and clearly label protocol
  changes before comparing tokens, review counts, or artifact counts.
- Direct performance comparisons across cohorts must be described as
  contextual, not controlled apples-to-apples measurements.
- Comparisons within the same cohort remain the authoritative measurements.
- The user-facing setup command should hide these implementation differences;
  the protocol metadata belongs in the result archive and analyzer report.

## Future benchmark telemetry capture

- Add a structured telemetry-capture item to the 1.4.2 benchmark protocol.
  Preserve per-worker and reviewer thread IDs, rollout/database provenance,
  phase boundary events, token composition and cache fields, reviewer
  durations and tokens, event/message/reasoning counts, tool-call and
  serialized input/output volume, validator attempts, patch/function-call
  counts, command read/write activity, and parent/child reviewer lifecycle
  records. Emit a validated machine-readable raw JSON artifact alongside the
  human comparison report, with exact versus heuristic fields and retention
  paths clearly marked. Define the detailed schema and extraction plan later
  from the legacy data already gathered; do not retrofit older benchmark
  cohorts.

## Missing benchmark-harness safeguards

- Add process-group cancellation handling to `setup-and-run.sh`: install
  signal traps before workers start, forward Ctrl+C to every active worker and
  reviewer descendant, wait for cleanup, remove temporary state, and return a
  distinct interrupted status. Verification must prove that no worker,
  reviewer, validator, or child process survives an interrupted run.
- Make telemetry discovery system-independent. Resolve the active telemetry
  database from configured/current locations, match records by exact worker
  UUID, reject stale or ambiguous matches, and record the selected database
  path plus lookup method in every result archive. Missing or mismatched
  telemetry must be reported as a data-integrity failure rather than silently
  substituted.
- Separate taint causes by layer: agent access violation, process-audit
  violation, missing telemetry, worker failure, reviewer failure, and analyzer
  failure. Preserve the raw evidence and prevent one layer's false positive
  from being reported as another layer's defect.
- Make result archival atomic and self-describing: stage one run directory per
  `{timestamp}-{run-name}`, include the benchmarked skill/version and protocol
  metadata, then publish only after all artifacts and telemetry checks pass.
- Require the analyzer to emit a short per-version developer journey with
  review rounds, findings, fixes, validation attempts, artifact expansion, and
  latency/token deltas versus the prior revision. Missing evidence must be
  labeled unavailable rather than inferred as zero.
