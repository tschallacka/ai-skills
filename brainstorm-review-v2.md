# Critical review v2: planning optimization brainstorm

## Executive verdict

The improved brainstorm is a materially better direction than the `.old`
brainstorm and the first review. It correctly narrows the cache experiment,
retains the existing Markdown/helper IDs as the intended authority, adds
explicit dependency tracking, refuses normal use of stale generated guidance,
and replaces the ambiguous `check --changed`/`refresh --changed` workflow with
diagnostic `--changed` plus atomic `refresh --stale`.

It is not yet implementation-ready. The cache portion is a plausible Phase 1
experiment, but global plan registration, plan-local Git, versions/changelogs,
external-edit quarantine, and cross-file recovery turn the proposal into a
multi-store transaction system. Those additions are described as safeguards,
not as an implementable protocol. The proposal should be approved only as a
design to refine and measure, not as authorization to build the whole feature.

The recommended boundary is:

1. implement and measure the four-command, dependency-aware cache over current
   document IDs;
2. specify registry/Git/version/quarantine as separate design work with crash,
   concurrency, and human-approval decisions resolved first; and
3. make every later capability conditional on measured benefit and an explicit
   recovery/test contract.

## What changed, and whether the change helped

The improved document intentionally removes or defers the first review's
largest scope risks:

- the P/G/S/A namespace, facts, deltas, compaction, and worker orchestration
  are deferred (`planning-optimization-strategies.txt:18-20,400-406`);
- the cache is limited to `init`, `read`, `check`, and `refresh`;
- selectors use existing document IDs and long flags;
- dependencies are per generated entry rather than one primary-file hash;
- index freshness, stale generated reads, bounded output, JSON versioning,
  fanout reporting, snapshots, locking, and mandatory `--all` audit triggers
  are now named;
- `check --changed` is diagnostic only and `refresh --stale` recalculates and
  refreshes under one lock (`planning-optimization-strategies.txt:155-180`).

These changes directly answer the first review's findings in
`brainstorm-analisis.md:107-295` and remove the old implementation order that
would have built facts/deltas before proving the cache (`planning-optimization-
strategies.md.old:400-415`). This is a coherent improvement.

The new material is not merely an implementation detail, however. The improved
draft adds a global identity registry, plan-local Git repositories, mandatory
helper commits, document versions, changelogs, quarantine incidents, external
human approval, tombstones, plan moves, and recovery states. These are visible
in `planning-optimization-strategies.txt:32-93,209-256,270-390`. They introduce
more state and more authority boundaries than the deferred features removed.
The proposal therefore reduces conceptual duplication in the cache while
substantially increasing operational complexity around mutation and recovery.

## Strong parts that should be retained

### Minimal dependency-aware cache

The cache scope is now correctly framed as processed generated entries, not a
claim that the whole plan is fresh. Explicit dependency sets, per-dependency
hashes, index/parser/schema markers, reverse fanout, bounded reads, and stale
read failure are the right minimum correctness model. The distinction between
“no processed dependencies” and “fresh processed scope” is especially important
(`planning-optimization-strategies.txt:107-151`).

The proposal also preserves a safe escape hatch: direct canonical reads remain
available when generated guidance is stale. That matches the planning skill's
requirement to use bundled helpers and keep canonical Markdown durable
(`planning/SKILL.md:41-54,174-195`).

### Audit boundaries and deferred event history

The required broader audits before completion, after decomposition/ownership/
dependency changes, after renames/deletions, and after parser/schema changes
are appropriate. Routine processed-scope checks can remain cheap, provided the
skill and validator actually enforce those boundaries. Deferring events and
compaction avoids recreating the first review's second-source-of-truth problem.

### Safety posture

The draft is right that a hash/version mismatch must not be “repaired” by an
agent merely to make the cache pass. Preserving the prior snapshot, blocking
affected writes, avoiding bypass flags, and requiring a human decision are
strong fail-closed principles (`planning-optimization-strategies.txt:320-364`).
The exact-scope Git staging rule and prohibition on reset/discard/broad staging
are also sound safeguards.

## Blocking issues before implementation

### 1. The Phase 1 boundary is internally inconsistent

Phase 1 says the command has only four commands, but the same proposal requires
plan creation/allocation, lookup, plan moves/repair, helper-managed commits,
source-helper invalidation, and later `changes` and human approval workflows.
Examples use `lookup` and `changes`, although neither is in the Phase 1 command
set (`planning-optimization-strategies.txt:26-30,83-93,298-305`).

More importantly, “plan creation allocates an ID before initialization” does
not identify which existing command performs allocation or how it composes with
`create-plan.sh`. The current skill requires `create-plan.sh` to create the plan
directory and canonical files, and says creation helpers own formatting and
initial state (`planning/SKILL.md:140-170`). The proposal must explicitly
choose one of:

- modify `create-plan.sh` to own registry allocation and initialization;
- add a separate plan-creation transaction that calls it; or
- leave global IDs out of the cache experiment.

Until then, `init` cannot be tested as specified.

### 2. Registry metadata creates a second authority unless its fields are reduced

The draft says Markdown remains the sole source of truth, but the registry
stores canonical path, title, and status, while plan metadata stores the plan
ID, and the cache index stores document-ID/path associations. It then requires
registry and plan metadata to agree before success (`planning-optimization-
strategies.txt:56-80`). That is replicated authority, not a cache.

The registry should be minimal: a never-reused plan ID, lifecycle state, and
canonical path (if global lookup is truly required). Title and plan status
should be derived from canonical documents/progress, not independently edited
registry fields. Define which copy wins during repair, how disagreement is
detected, and how a moved plan is found after an external move. Otherwise a
registry path pointing to the old location makes the proposed repair command
unreachable.

Also resolve the scope of “global.” The planning skill's plans root is an
installed skill directory (`planning/SKILL.md:140-157`), potentially outside
this repository and shared by multiple workspaces/users. The proposal needs
permissions, portability, backup, schema migration, and ownership rules for
that global file.

### 3. Atomicity across registry, canonical files, cache, and Git is not defined

An atomic directory switch makes one cache generation consistent, but it does
not make these stores one transaction:

`registry rewrite -> canonical mutation/version/changelog -> cache snapshot ->
Git index/object/commit`.

Git commits cannot be rolled back atomically with an external registry file or
a cache pointer. The draft acknowledges “recoverable invalid state” but does
not define a journal, transaction ID, phase markers, replay rules, idempotency,
or repair commands (`planning-optimization-strategies.txt:241-251,385-390`).

Concrete unresolved crashes include:

- ID allocated, plan files created, process dies before registry publication;
- registry published, initial commit fails;
- canonical mutation and version succeed, cache publication fails;
- cache publication succeeds, Git commit fails;
- Git commit succeeds, registry/path update fails;
- a commit hook changes or rejects files after staging.

“One logical mutation” is not a recovery algorithm. Specify the durable state
machine and recovery owner before claiming atomicity. If that cannot be made
small, keep the cache transaction separate from source mutation and defer
helper-managed commits/versioning.

### 4. Plan-local Git introduces unresolved nested-repository behavior

The draft correctly notices nested repositories, but “detect and clearly
report” does not say whether initialization proceeds, warns, or refuses
(`planning-optimization-strategies.txt:32-46`). This matters because the current
skill normally places plans under an installed `plans/` root, which may itself
be inside a parent repository.

Define behavior for: parent repository detection, an existing `.git` directory
versus a `.git` file/worktree/submodule, pre-existing untracked files,
pre-existing staged changes, configured Git identity, hooks, ignored files,
and parent status visibility. Define which exact canonical files belong in the
initial commit and whether generated context is committed. A local repository
can be a useful history boundary, but it should not be a prerequisite for the
minimal cache experiment unless the project explicitly accepts this operational
cost.

The “every mutating helper command creates one commit” rule also conflicts with
the planning skill's multi-file creation helpers and generated progress updates
unless the commit boundary, exact file list, empty-change behavior, and failed
commit recovery are specified for each helper. The maintainer contract requires
every hard rule to have a regression assertion (`planning/MAINTAINER-STYLE-
CONTRACT.md:1-7`); the proposal names no such concrete fixtures or tests.

### 5. Locking is named but not designed

There are at least three lock domains: the global registry, the plan context,
and source/helper/Git mutation. The draft does not define lock ordering,
reentrancy, timeout, stale lock recovery, or whether a reader hashes source
files while holding the source mutation lock. A shared cache lock does not
prevent a canonical file from changing halfway through dependency hashing.

It also does not define the interaction with Git's own index lock or hooks.
Without a single consistent order, two normal operations can deadlock (for
example registry then plan versus plan then registry); without an ownership and
recovery protocol, a killed process can leave the plan permanently blocked.
Specify lock scope and ordering, or use one plan transaction lock plus a
separate registry allocation lock with a documented protocol.

### 6. Quarantine and human approval are safe in intent but not implementable

The draft requires an “out-of-band human approval recognized by the execution
environment,” but no mechanism, credential, file/API boundary, actor identity,
expiry, audit record, or recovery command is defined. An agent cannot invent a
flag, which is good, but an implementation cannot authenticate an undefined
approval (`planning-optimization-strategies.txt:353-364`).

There are also internal contradictions:

- incident records are called append-only, but repeated attempts update a
  bounded attempt count;
- approved acceptance records `external-change-approved`, but the permitted
  operation list only includes `external-change`;
- the helper must preserve the prior snapshot and not mutate the changelog, yet
  it must record an incident somewhere and later atomically add a changelog
  entry;
- the rule assumes every external uncommitted edit is a possible rogue-agent
  edit, but does not define how a legitimate human first edit is identified or
  how a human recovers without the blocked helper.

Define an immutable incident ID and append-only event records, a bounded view
rather than an updated append-only record, and a real approval adapter. Define
restore and accept transactions, including what happens to the external Git
commit, the source hash, version allocation, cache generation, and incident
closure. Until then, quarantine is a policy statement, not an executable
workflow.

### 7. Phase 1.5 versions and changelogs need a document model

The draft correctly keeps versions in addition to hashes and keeps changelog
output out of default reads. However, it does not say where version metadata
lives in the current canonical formats, how existing documents are migrated,
or how helper-generated multi-document changes allocate versions. A single
`add-work-unit.sh` operation changes at least the inventory and a step; a goal
creation may create several documents. “Exactly one version per changed
document” needs ordering and transaction semantics.

Version advancement on a failed Git commit is particularly unclear: the draft
says a failed commit leaves `commit-pending`, but does not say whether the
version/changelog are committed, pending, restored, or eligible for human
completion. It also says a source mutation can advance its version while its
cache remains stale, which is reasonable, but the corresponding manifest and
Git state must be described precisely.

The current contract prohibits hand-edited paragraph numbering and requires
updates through flagged helpers (`planning/MAINTAINER-STYLE-CONTRACT.md:20-34`).
Adding version fields or history records therefore requires changes to the
creator, parser, mutator, validator, fixtures, and contract—not just a cache
manifest. The improved document acknowledges this only at a high level.

## CLI, token cost, and correctness gaps

The long-flag decision is good, but the grammar remains incomplete. It needs a
subcommand-by-subcommand table defining required selectors, mutually exclusive
selectors, whether `--plan-id` and `--plan-dir` may coexist, default views,
entry IDs versus document IDs, treatment of deleted entries, and the exact
meaning of `refresh --entry` versus `refresh --stale`.

The proposal still has a token-cost trap: a routine resume may become
`check -> refresh --stale -> read`, even when `read` could perform one bounded
freshness check and return the view. Make the normal path one command where
possible, keep refresh output metadata-only, and measure model-visible tokens,
not just shell work. “Material benefit” has no threshold, baseline, sample
plans, or stopping rule (`planning-optimization-strategies.txt:392-398`).

The fixed JSON schema and truncation requirement are good, but no schema,
continuation-token lifetime, maximum defaults, or behavior for a source that
changes during pagination is given. Path-based dependency manifests also need
stable path normalization and protection against symlink/path replacement.

## Missing acceptance tests

The acceptance list is a useful checklist, but it is not yet an executable
acceptance suite. In particular, tests must cover at least:

- two concurrent ID allocations, allocation gaps, failed creation, duplicate
  registry rows, permissions, and crash recovery;
- plan creation inside a parent repo, existing nested repo/worktree, dirty
  files, hooks, missing Git identity, exact staging, and unrelated changes;
- registry/metadata/path mismatch, helper move, external move, rename,
  deletion, replacement, and tombstone non-reuse;
- source mutation during `check`, refresh, read registration, and snapshot
  publication; lock contention, timeout, stale-lock recovery, and lock-order
  deadlock prevention;
- every crash point in registry/canonical/cache/Git sequencing, with a
  deterministic repair result;
- version allocation across multi-document helpers, failed commits,
  changelog bounds, rename history, and migration of old plans;
- external edit quarantine, repeated alternate attempts, authenticated reject
  and accept flows, incident immutability, and inability to emit suspect
  content as trusted guidance;
- stale generated reads, direct canonical reads, empty processed scope,
  index changes, parser/schema changes, fanout limits, deleted dependencies,
  JSON schema, truncation, continuation, and stable exit categories; and
- a token benchmark comparing the existing helper workflow with cache-enabled
  resume sessions across small, medium, and highly coupled plans.

These should become concrete fixtures, commands, expected exit/status values,
and named test work units. That follows the planning skill's rule that every
hard rule needs regression coverage and that implementation steps must name one
concrete target (`planning/SKILL.md:74-104,307-347`).

## Recommended disposition

Approve the cache experiment after tightening its CLI grammar and acceptance
tests. Keep canonical Markdown and existing `plan`, `goal:<goal>`,
`step:<goal>/<step>`, and `unit:<WNN>` IDs as the only content identity.

Defer global IDs, plan-local Git, versions/changelogs, and quarantine until
each has a separate design decision for authority, transaction state, locking,
approval, migration, and recovery. If global plan lookup is essential, first
reduce the registry to an explicitly derived lookup service and define its
rebuild/repair semantics. If local Git is retained, treat commits as an audit
history boundary rather than pretending Git and filesystem writes are one
atomic transaction.

The improved brainstorm is coherent at the architectural-principles level and
substantially safer than both predecessors. It is incomplete at the protocol
level, over-scoped for a first measured experiment, and not safe to implement
without resolving the issues above.
