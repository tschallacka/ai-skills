# Critical Review: Planning Skill Optimization Brainstorm

## Executive assessment

The brainstorm identifies real costs—re-reading plan documents, carrying completed context, and silently using stale generated guidance—but its proposed solution is too large and internally under-specified for a first implementation.

The strongest idea is selective freshness over a processed set. The weakest parts are the new marker namespace, the broad `plan-context.sh` command surface, the proposed fact/delta database, and the assumption that file hashes alone can establish semantic freshness.

The architecture should be reduced to:

1. Existing canonical planning documents remain authoritative.
2. Existing document and work-unit IDs remain the public selectors.
3. A small generated cache records explicit source dependencies and hashes.
4. `check`, `refresh`, and bounded `read` are implemented first.
5. Facts, deltas, workers, and compaction are deferred until measured usage justifies them.

## What is sound

The following principles align well with the planning skill:

- Agents should use bounded reads and avoid loading unrelated documents.
- Generated context should not be authoritative.
- Mutations should go through bundled helpers.
- Generated files should be written atomically.
- Selective freshness is preferable to hashing or rereading every plan file on every resume.
- Freshness checks should distinguish processed material from genuinely unread material.
- A fresh secondary review is appropriate for a material planning-system change.
- The coordinator/worker warning is valid: host-level compaction may terminate the active agent, so compaction should not be treated as an ordinary continuation operation.

The brainstorm is also correct that hashes detect file changes, not semantic correctness. That caveat should be elevated from a final safeguard to a central architectural constraint.

## Major contradictions and ambiguities

### 1. Stable markers conflict with renames and index regeneration

The proposal says markers are stable for the lifetime of a plan, but also says that renaming a source file removes the old marker and creates a new mapping.

Those are different guarantees:

- A stable marker must continue identifying the same logical object after a rename.
- A removed marker means references to it become invalid.
- A new marker means downstream dependencies must be rewritten or resolved through an alias.

The proposal does not define whether markers identify:

- a file,
- a logical work unit,
- a document section,
- or a generated index row.

The safer rule is:

> Stable IDs identify logical plan objects. File paths are mutable attributes of those objects.

A rename should preserve the work-unit or document ID and update its path. If an object is genuinely deleted, its ID should become tombstoned and never be reused. The index should not create a new logical identity merely because a path changed.

### 2. The new P/G/S/W/A namespace duplicates the planning skill’s existing identity model

The planning skill already defines:

- `plan`
- `goal:<goal>`
- `step:<goal>/<step>`
- `unit:<WNN>`

The brainstorm introduces:

- `P55`
- `G03`
- `S04`
- `W07`
- `A12`

This creates two identity systems with unclear conversion rules. In particular:

- Is `P55` a globally indexed plan or a plan-local ID?
- Are `G03` and `S04` numeric indexes or stable identifiers?
- Does `A12` identify a file, a section, or an artifact relationship?
- Can two plans contain the same `W07`?
- What happens when goals are reordered?
- What happens when a goal is split?
- Does an artifact retain its marker after moving between work units?

The existing document IDs are more explicit and already supported by `plan-content.sh`. The new system should either reuse them or define a strict, tested compatibility layer. Introducing short aliases without a migration and uniqueness contract will make diagnostics harder to understand and will likely cause stale references.

### 3. Numeric selectors are not sufficiently defined

The command examples mix numeric selectors and stable markers:

```text
read -p 55 -g 3 -s 4
read -p 55 -w W07
read -p 55 -a A12
```

The proposal does not specify whether `-g 3` means:

- the third goal by current order,
- goal ID `03`,
- a global goal number,
- or a display alias.

If it means current order, inserting a goal changes the meaning of existing commands. That violates stable lookup expectations. If it means a stable ID, the command should accept the stable ID directly rather than hide it behind a number.

The same problem applies to step numbers. Numeric hierarchy indexes are convenient for humans but unsafe as durable references.

### 4. “Read registers the artifact” is not enough for derived reads

A compact resume read may depend on several sources:

- plan description,
- progress,
- work-unit inventory,
- goal document,
- step document,
- working context,
- completed handoffs,
- index metadata.

The brainstorm sometimes says the read registers the “resolved source artifact,” sometimes says a summary hashes “the inventory/progress artifacts used,” and sometimes says only the source document containing a section is hashed.

This is not a complete dependency model. A generated result is fresh only if all inputs used to produce it are fresh.

The cache must record a dependency set per generated entry, not merely one artifact hash. For example:

```text
cache-entry: goal:checkout
depends-on: plan-description.md, work-unit-inventory.md, goal.md, progress.md
```

Otherwise a changed progress tracker or completed handoff can leave a cached resume summary apparently fresh.

### 5. The index itself has no freshness contract

The index is generated from source documents, but `check` only compares processed artifacts in `processed.sha256`. The proposal does not explain how it detects:

- a new goal,
- a deleted step,
- a changed work-unit ownership row,
- a changed path,
- a changed document ID,
- or a malformed index.

If the index changes while the processed source hash does not, selectors can resolve differently without a stale result. Exit code `2` only covers missing or bad files, not a valid-but-outdated index.

The index needs either:

- its own input manifest and freshness check, or
- deterministic regeneration before every lookup, with bounded output and no token exposure.

The second option is simpler if generation is cheap.

### 6. `--changed` and `refresh --changed` are operationally ambiguous

The proposed workflow is:

```text
check --changed
refresh --changed
```

But the second command has no defined durable source for the marker set. It could:

- rerun the check,
- read temporary state,
- read the previous command’s output,
- or calculate a different result.

Each option has problems:

- Rerunning can produce different results if files change between commands.
- Temporary state creates concurrency and cleanup issues.
- Shell output is not a safe state channel.
- A second calculation can disagree with the first.

The command should instead support one explicit operation:

```text
refresh --stale
```

where refresh performs one freshness calculation and refreshes the resulting set under a lock. Alternatively, require explicit marker arguments and keep `check --changed` purely diagnostic.

### 7. Marker output does not map cleanly to refresh arguments

The example stale output is:

```text
stale P55
changed W07 A12
removed A09
action: plan-context.sh refresh -p 55 -w W07 -a A12
```

This loses information:

- `P55` is not a valid refresh selector by itself.
- `A09` is reported as removed but cannot be refreshed.
- Multiple artifacts may share a work unit.
- A changed parent document may require refreshing several derived entries.
- The output does not distinguish changed source files from affected cache entries.

The system needs separate concepts:

- changed source objects,
- affected generated entries,
- removed source objects,
- selectors accepted by refresh.

Those should not be compressed into one marker line.

## Freshness and correctness risks

### 1. Hashes do not capture dependency meaning

A file hash detects bytes changing, but not necessarily the meaning of a plan relationship. Examples include:

- A work unit remains byte-identical while ownership changes in another document.
- A goal is moved under a different dependency.
- A path remains present but points to a different generated artifact.
- A source file is rewritten with equivalent content but a changed external dependency.
- A validator or parser changes, making previously generated context incompatible.

The cache should include a generator/schema version and dependency metadata. Any change to the parser, helper, or output schema should invalidate relevant generated context.

### 2. Unread-source exclusion can hide scope changes

Ignoring unread files is reasonable for normal resume cost, but it creates a dangerous distinction between:

- “not relevant to the current task,” and
- “not yet discovered to be relevant.”

A newly added dependency, risk, handoff, or work unit can remain invisible if it is outside the processed set. The brainstorm mentions `--all`, but does not specify when an agent must use it.

The revised design should define mandatory audit triggers, such as:

- before declaring a goal complete,
- after a plan decomposition change,
- after a dependency or ownership change,
- before a fresh adversarial review,
- after validator or helper schema changes,
- when a processed source is removed or superseded.

Selective freshness should optimize routine resumes, not replace scope validation.

### 3. Stale content can still be read

The brainstorm says stale context cannot be read until selected markers are refreshed, but it does not define enforcement boundaries.

Questions left open:

- Does `read` fail if any dependency is stale?
- Does it return stale content with a warning?
- Does `--read-only` bypass the block?
- Does `--full` bypass it?
- Can a direct source-document read proceed while generated resume content is stale?
- What happens if only one section of a multi-section result is stale?

The safe default is:

- Generated cache reads fail with a machine-readable stale result.
- Direct source reads remain available.
- `--read-only` may inspect stale generated content only with an explicit unsafe flag, preferably not at all.
- A stale result must identify the exact source dependencies and avoid emitting stale guidance as normal content.

### 4. Atomic writes are not atomic multi-file transactions

The proposal correctly requires temporary files and renames, but commands such as refresh and compact update several related files:

- resume cache,
- facts,
- deltas,
- manifest,
- index,
- possibly progress-derived data.

Separate atomic renames can still leave a mixed generation if the process is interrupted between files. The design needs either:

- one versioned snapshot directory switched by a single rename,
- a transaction journal and recovery protocol,
- or a clearly stated consistency model.

Without this, a cache can contain a new resume file paired with an old manifest and appear fresh incorrectly.

### 5. Concurrent agents are not addressed

The coordinator/worker model makes concurrent access more likely, yet there is no locking policy. Potential races include:

- two workers appending deltas,
- refresh and mutation running simultaneously,
- one process checking while another rewrites a source,
- two compactions folding the same deltas,
- a read registering a hash during a source mutation.

Atomic rename alone does not prevent lost updates or inconsistent snapshots. The helper needs an explicit locking strategy and conflict behavior.

## CLI and API problems

### 1. The command surface is too broad for an initial release

`init`, `read`, `check`, `refresh`, `add-fact`, `add-delta`, and `compact` collectively create a second planning system. Each command requires:

- parsing rules,
- validation,
- compatibility behavior,
- locking,
- atomic writes,
- tests,
- schema evolution,
- error codes,
- documentation,
- migration behavior.

This is a large implementation project before the core token-saving hypothesis has been measured.

The first version should implement only:

- bounded read,
- dependency-aware check,
- selective refresh,
- cache invalidation after helper mutations.

Facts, deltas, and compaction should follow evidence of an actual context-history problem.

### 2. `init` can produce a misleadingly valid empty state

`init` creates context files but does not process artifacts. A subsequent check reports fresh because nothing has been processed. That is technically consistent with the processed-set model but operationally easy to misread as “the plan is fresh.”

The output should distinguish:

```text
fresh: no processed dependencies
```

from:

```text
fresh: 12 processed dependencies checked
```

A new plan should perhaps begin with a compact baseline read, or initialization should explicitly state that freshness has not yet been established.

### 3. `read` behavior with no section flag is unclear

The examples imply that:

```text
read -p 55 -g 3 -s 4
```

returns compact active context, while section flags select additional content. But the rules do not define:

- which sections are included by default,
- whether `-O` and `-T` replace or supplement the default,
- whether `-N` is valid without a goal/step,
- what an artifact read returns,
- whether multiple selectors are a union or a constrained path,
- how duplicate sections are handled.

These must be specified as a grammar, not inferred from examples.

### 4. `--full` is semantically contradictory

The proposal says `--full` is opt-in but “returns the selected flagged sections without expanding unrelated sections.” That is not obviously different from a normal flagged read.

Possible meanings include:

- return full source documents,
- return full paragraphs instead of compact summaries,
- return all sections for the selected object,
- disable output truncation.

Only one should be chosen. A name such as `--raw-source` or `--all-selected` would be clearer if that is the intended behavior.

### 5. Short-flag collisions are only partially solved

The brainstorm correctly notices that dedicated section flags need collision testing, but it does not present a complete option table.

Potential ambiguity includes:

- `-p` as plan selector versus existing paragraph-style `-p`,
- `-a` artifact versus any future “all” shorthand,
- `-f` format versus future file selection,
- `-C` acceptance criteria versus common config conventions,
- `-D` dependencies versus debug,
- `-R` risks versus raw,
- `-V` verification versus verbose,
- `-N` next versus numeric.

The short flags are not necessary. Long flags are more readable, safer for future expansion, and more appropriate for agent-generated commands. If short flags are retained, they need a complete reserved namespace and regression tests for every subcommand.

### 6. Error codes are incomplete

The three exit codes are useful but insufficient for automation. The helper should distinguish at least:

- usage error,
- missing selector,
- unknown marker,
- invalid combination,
- stale cache,
- malformed context,
- source missing,
- lock contention,
- write failure,
- internal parser failure.

If the project wants a small stable code set, the output must carry a machine-readable error category. Otherwise agents will be forced to parse prose.

### 7. JSON output is underspecified

The proposal offers `text`, `markdown`, and `json`, but does not define schemas. This is especially risky for:

- marker arrays,
- stale versus removed states,
- source dependencies,
- errors,
- partial reads,
- schema versioning.

Generated JSON needs a version field and a fixed schema. “JSON” should not mean a serialization of whatever internal structure currently exists.

## Facts, deltas, and compaction are under-designed

### 1. Facts lack provenance and invalidation

A fact such as:

```text
current-route=/checkout
```

needs more than a key and value. It should record:

- when it was observed,
- who or what confirmed it,
- related source or work-unit marker,
- confidence or status,
- whether it is still valid,
- whether it supersedes another fact.

Otherwise `facts.tsv` becomes an unreviewed second source of truth.

### 2. Duplicate replacement semantics are awkward

The proposal says `add-fact` can record or replace a fact, but duplicate replacement requires `--replace`. This is defensible, yet it creates a race:

1. check whether key exists,
2. decide whether to use `--replace`,
3. write the file.

The helper must perform existence validation and replacement atomically. It also needs a behavior for stale facts and conflicting values.

### 3. Delta events duplicate existing planning artifacts

Discoveries, decisions, blockers, verification results, and handoffs already have natural homes in working context, progress, step handoffs, and adversarial review documents. Adding NDJSON may improve append-only logging, but it also creates competing records.

The design needs a clear rule:

- Which information belongs in deltas?
- Which information must be promoted into canonical plan documents?
- Does a delta affect completion status?
- Can a plan be considered complete while relevant deltas remain unreviewed?

Without this boundary, agents will log events but fail to update the authoritative plan.

### 4. Hash snapshots in every delta are a token and storage trap

Including the “relevant processed-hash snapshot” in every event can become expensive and redundant. A long-running initiative may produce hundreds of events, each carrying repeated hashes.

Use a compact snapshot ID or generation number, with the full manifest stored once. A delta should reference:

```text
snapshot: 17
```

rather than duplicate the entire hash state.

### 5. Compaction has no safe semantic rule

“Fold confirmed deltas into facts” assumes a confirmation model that is not defined. It also raises conflict questions:

- Which event kinds are foldable?
- Who confirms them?
- What if two decisions conflict?
- What happens to blockers?
- How are superseded facts retained?
- Does compaction rewrite history or merely mark events as folded?
- Why should compaction update processed source hashes?

Compaction should not be implemented until event semantics and audit requirements are explicit.

## Token-cost traps

### 1. The proposed workflow may add more calls than it removes

The normal path becomes:

```text
check
check --changed
refresh --changed
read compact context
read section
possibly read dependency
add-delta
check again
compact
```

This can cost more tokens and process overhead than one bounded `plan-content.sh` read, especially for small goals.

The design needs measured thresholds, not an assumption that selective operations are always cheaper.

### 2. Machine-readable output can still be large

JSON frequently costs more tokens than compact text because it repeats field names and structural punctuation. A `--full` JSON result could be substantially more expensive than the source Markdown it replaces.

The helper should support:

- hard output-size limits,
- record counts,
- pagination or continuation tokens,
- compact field names only if the schema is stable,
- explicit truncation indicators,
- no duplicated path and description fields unless requested.

### 3. Refresh may duplicate source consumption

If `refresh` re-reads a document internally and emits the refreshed content, then the agent may consume the document once during refresh and again during `read`. If refresh emits only a compact status, the agent may still need a subsequent read.

The intended token accounting should be defined:

- helper-internal shell processing is cheap,
- model-visible output is expensive,
- refresh should normally emit only changed markers and cache metadata,
- the following read should return the required content once.

### 4. Dependency refresh can fan out unexpectedly

A small change to a plan description or inventory may invalidate many derived entries. The proposal says “rebuild only affected compact resume entries” but does not define fan-out limits or output behavior.

A refresh should report:

- number of changed sources,
- number of invalidated cache entries,
- whether dependent entries were rebuilt,
- whether the result was truncated.

### 5. Worker boundaries may increase rediscovery cost

The brainstorm correctly acknowledges this, but “one worker per goal” remains too broad as a default. It can be more expensive when:

- goals are tiny,
- the same repository symbols are needed repeatedly,
- goals share a large prerequisite handoff,
- setup and environment discovery dominate execution,
- workers must rediscover unresolved risks.

Worker splitting should be based on measured context size and coupling, not only goal boundaries.

## Architectural recommendation

### Phase 1: Minimal dependency-aware cache

Keep the authoritative model unchanged:

- plan documents,
- work-unit inventory,
- progress trackers,
- handoffs,
- existing helper-generated IDs.

Add one generated cache directory with:

- a versioned index,
- a dependency manifest,
- compact cached entries,
- source hashes,
- a generator/schema version.

Do not add facts, deltas, or new marker namespaces yet.

Use existing selectors wherever possible:

```text
plan-content.sh get <plan-directory> unit:W07 json
plan-content.sh get <plan-directory> goal:01-foundation markdown
plan-content.sh summary <plan-directory> text
```

If a new helper is needed, prefer explicit identifiers:

```text
plan-context.sh read --plan-dir <dir> --document goal:01-foundation
plan-context.sh read --plan-dir <dir> --unit W07
```

Avoid numeric positional goal and step indexes.

### Phase 2: Explicit dependency tracking

Each generated cache entry should record:

- entry ID,
- source paths,
- source hashes,
- parser/helper schema version,
- generated timestamp,
- cache state.

A cache entry is usable only when all dependencies match. Source mutations performed through planning helpers should invalidate known dependent entries immediately; a later check remains the recovery mechanism.

### Phase 3: Small, deterministic command set

Start with:

```text
init
read
check
refresh
```

Define them precisely:

- `read` returns bounded content or a stale error.
- `check` checks the selected cache scope.
- `refresh` refreshes explicit stale entries or an explicit scope.
- `init` refuses overwrite and clearly reports that no dependencies have yet been processed.

Prefer long flags. If `--changed` is retained, make it diagnostic only. Use `refresh --stale` for an atomic recompute-and-refresh operation, or require explicit IDs.

### Phase 4: Snapshot consistency and locking

Use a versioned snapshot directory:

```text
context/snapshots/17/
context/current -> snapshots/17
```

Write the new snapshot completely, validate it, then switch the current pointer atomically. Protect source mutation and cache updates with a lock. If symlinks are unsuitable, use a generation file and atomic directory replacement.

### Phase 5: Mandatory audit boundaries

Require a broader audit when:

- a plan’s scope changes,
- ownership or dependencies change,
- a goal is being completed,
- a source is deleted or renamed,
- the parser or cache schema changes,
- a fresh adversarial review is requested,
- the validator reports structural changes.

Routine resume checks may remain processed-set-only.

### Phase 6: Add events only after evidence

If append-only execution history is still needed after Phase 1 is measured, add deltas with:

- a schema version,
- event ID,
- timestamp,
- kind,
- message,
- related existing document/work-unit ID,
- snapshot ID,
- provenance and promotion status.

Do not let deltas silently become authoritative. Define how they are promoted into plan documents and how unresolved events affect completion.

Defer compaction until there is a tested retention and audit model.

## Minimum acceptance criteria for the revised design

Before implementation is considered ready, the plan should specify and test:

- stable logical IDs surviving file renames;
- no ID reuse after deletion;
- exact selector grammar;
- exact default `read` output;
- behavior for stale generated content;
- dependency sets for every generated entry;
- index and schema invalidation;
- atomic multi-file snapshot behavior;
- concurrent mutation and refresh behavior;
- source deletion and replacement;
- check/refresh race behavior;
- bounded output and truncation signaling;
- machine-readable error categories;
- long-option collision rules;
- fresh versus empty processed-set reporting;
- audit triggers for scope changes;
- compatibility with the planning skill’s existing document IDs and helper commands.

## Final verdict

The brainstorm contains a promising optimization, but the current proposal should not be implemented as a single feature. It combines cache invalidation, a new identity system, a document index, an event log, fact storage, compaction, worker orchestration, and a new CLI before proving that the basic cache saves tokens.

Approve the selective dependency-aware cache as a focused experiment. Reject the new P/G/S/A identity layer and the facts/deltas/compaction subsystem until their semantics are independently specified and measured. The best architecture is a small, versioned, dependency-tracked generated cache layered over the existing planning helpers and document IDs, with explicit audit boundaries and no second source of truth.
