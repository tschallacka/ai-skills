# Reviewer contract

> Generated from `SKILL.md` by `scripts/generate-reviewer.sh`.
> Reviewer profile contract: `1.4.2`
> Source SHA-256: `865d7e926217a14f105f2999dcb754985bba4a09683638fd0ecebb07447e935d`

This file is a review-scoped projection of the tagged `SKILL.md`; the tagged skill remains authoritative.

## Generated sections

- `mandatory-review`
- `bounded-context`

## 3. Mandatory classification and independent review

`create-plan.sh` creates the mandatory UI classification and adversarial-review
sections. Keep both concise and update structured values through the CLI.

When `UI affected: yes`, the UI validation reference applies and its `Required`
value must be `yes`. Never use `UI affected: no` to avoid browser validation
for an HTML, CSS, template, component, route, or user-facing behavior change.

Before a plan is ready to execute, a **fresh secondary agent** must create
`adversarial-review.md`. Give that agent the request, repository context, and
plan artifacts, but not the planning agent's conclusions. It must identify
every unplanned file, symbol, behavior, test, browser interaction, dependency,
and bug-recovery path needed to execute the request. The plan is rejected
until every finding is resolved and the review verdict is `✅ approved`.

The fresh adversary must be **bounded-read locked**. Hand it the exact reader
command, plan directory, and supported entry ids/views. Its starting prompt
must include verbatim:

"Read plan files and artifacts ONLY through the gated reader:
  bash <PLANNING_SKILL_DIR>/scripts/plan-context.sh read --plan-dir <PLAN_DIR> --document ID
  bash <PLANNING_SKILL_DIR>/scripts/plan-context.sh read --plan-dir <PLAN_DIR> --unit WNN
Valid --document IDs: plan, inventory, progress, adversarial-review,
goal:<goal id>, step:<goal>/<step>. Prefer the default summary view; raise
--max-records for a large inventory if needed. Plan-read bytes are capped at
the per-role budget when ROLE_ID is set (the gate lowers --max-bytes to it) —
do not rely on --max-bytes above that cap. Never load a whole
plan file, an entire plan directory, or the `.plans/` tree wholesale. A
wholesale file read of a plan artifact is a context-overflow violation. If the
gate cannot give you something, report it as a limitation — do not bypass it."

The fresh adversary assumes the **chris placeholder persona** (oriented scout):
spawn it with `ROLE_ID=chris`, have it load its scoped role docs and voice via
`bash <PLANNING_SKILL_DIR>/scripts/role-context.sh chris` (which injects its
stance preamble), and require it to state its persona id in the returned
findings. The adversary forms its own findings from the bounded-read gate and
its scoped role docs; it never receives the planning agent's conclusions. A
spawn that cannot resolve ROLE_ID=chris fails closed (the reader refuses) and
must be respawned with a valid identity.

Do not hand the adversary a command that dumps a plan file or directory in
full. Require the adversary's returned findings to state that all plan reads
went through the gate and to list any wholesale read it performed, so a
violation is visible (soft audit).

Do not allow the planning agent to approve its own review. Re-run the review
after a material scope change or a discovered bug.

The reviewer protocol is version `1.4.2`. Fresh-review mode remains the
default. Iterative mode is opt-in and must be bounded by a maximum of three
verification passes per reviewer and three fresh-review cycles per benchmark.
Reviewer records use `review_cycle`, `reviewer_session`, `finding_owner`,
`verification_pass`, `closed_findings`, `reviewer_handoff`, and
`review_mode`. Reviewer A may close only findings it owns and may not issue
overall plan approval. Reviewer B must perform the final independent review
and write one reviewer-owned `approval.json` containing
`reviewer_session_id`, `mode`, `approved_findings`, `rejected_findings`,
`approved_at`, and boolean `overall_plan_approval`. `false` is valid terminal
review evidence for detection grading, but it is never an adoption pass. Each
finding uses a stable `AR-NN` ID and records precise file/section evidence,
impact, observed contradiction, and required correction. A finding may
consolidate multiple defects; one finding per defect is not required. A fresh
reviewer must use a new session and capsule, receive no prior conclusions,
and perform the final independent approval.
Exceeding a pass or cycle limit, inheriting prior conclusions, or changing the
task contract or safety boundary marks the run unresolved and requires a fresh
review.

The review boundary is filesystem-enforced: each worker and reviewer receives
only its capsule and workspace, and each fresh reviewer receives a newly built
capsule. The capsule manifest, lifecycle records, and audit events are part of
the retained evidence. Missing identity, provenance, lifecycle, or
independence evidence is a publication failure rather than an inferred pass.

When the secondary review approves the plan, synchronize both status fields in
one atomic command (only after the independent reviewer has actually approved
it):

```bash
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --review-status \
  <plan-directory> approved
```

Use `--review-status <plan-directory> pending` when reopening the review. The
approved form refuses to proceed while the review still contains an open or
in-progress `AR-` finding. The validator rejects a missing, pending, or
mismatched plan-description status.

Execution order is mandatory: write the complete draft plan first, invoke the
fresh reviewer, wait for its artifact, resolve every finding by revising the
plan, then invoke a fresh reviewer again when revisions were material. Only
after an approved artifact exists may the planning agent run the readiness
validator and create progress trackers.

After review, revise only the named document target with the flagged update
commands. Use `-dp`/`--description-paragraph`, `-gp`/`--goal-paragraph`,
`-sp`/`--step-paragraph`, or `-rp`/`--review-paragraph` for one paragraph; use
the corresponding `-ds`, `-gs`, `-ss`, or `-rs` flag for a section with one or
more `-p N.N: content` paragraphs. For example:

```bash
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" \
  --description-paragraph <plan-directory> 6.1 "<revised affected area>"
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" \
  --step-section <plan-directory> 01-build/02-step-verify acceptance-criteria \
  -p 6.1: "<updated pass/fail criterion>"
```

Use `--table-paragraph <plan> <document-id> <N.N> <columns> "<CSV>"` to
replace one paragraph with a validated Markdown table. Quote the CSV fields;
use doubled quotes (`""`) for CSV-standard literal quotes, or `\"` when a
shell-friendly escaped quote is clearer. Use
`--insert-after` or `--insert-before` with a document ID and paragraph label
to add one paragraph; later labels in that same section shift automatically.

Run the validator again after revisions and reopen the adversarial review when
the change affects scope, ownership, dependencies, or acceptance criteria.


### 4.1 Bounded context and portable plan storage

Use the bundled `plan-context.sh` command for bounded reads instead of
repeatedly loading whole plan directories. Initialize a snapshot, read by
tagged identifiers, and check only entries that have been processed:

```bash
"$PLANNING_SKILL_DIR/scripts/plan-context.sh" init --plan-dir "$PLANS_ROOT/<plan>"
"$PLANNING_SKILL_DIR/scripts/plan-context.sh" read --plan-dir "$PLANS_ROOT/<plan>" --unit W55 --view instructions
"$PLANNING_SKILL_DIR/scripts/plan-context.sh" check --plan-dir "$PLANS_ROOT/<plan>" --changed
```

Hash drift is reported as `suspect`/`external-edit`; it is not automatically
overwritten or repaired. Ask for human resolution before refreshing. Agents
invoke the shell helpers; helpers own snapshot, state, and plan-file writes.

**All plan reads go through the gated readers.** Both the planning agent and
every fresh subagent (adversarial review, reviewer) must read plan documents
via the bundled gated readers (`plan-context.sh`), never by loading a whole
plan file or plan directory. Wholesale `Read`/`cat` of a plan artifact is a
context-overflow violation, because the gated readers strip plan metadata and
metadata-bearing front-matter that is not needed to act. Subagents receive this
instruction in their starting prompt (see section 3).
Plans use `PLANS_ROOT` when set, otherwise the home directory plus `.plans`,
with `USERPROFILE` and `HOMEDRIVE`/`HOMEPATH` support for Windows-compatible
Bash. On the constrained VPS used during this initiative, work sequentially,
keep at most one subchat active, and close it when complete; this is not a
generic restriction on other machines.

The Phase 1 command contract is fixed: `init` takes only `--plan-dir`; `read`
takes exactly one `--document` or `--unit` plus optional `--view`, `--format`,
`--max-bytes`, `--max-records`, and `--read-only`; `check` takes exactly one
of `--entry`, `--changed`, or `--all`; and `refresh` takes exactly one of
`--entry` or `--stale`. Defaults are `summary`, `text`, 32768 bytes, and 128
records. `--all` audits without registering entries. Global IDs, Git history,
versions/changelogs, quarantine, events, compaction, and workers remain
explicitly deferred from this Phase 1 cache.

At completion boundaries, run the non-registering audit and bounded experiment:

```bash
bash planning/tests/test-plan-context.sh --audit-triggers
bash planning/tests/test-plan-context.sh --benchmark
```

Treat the benchmark as a measurement gate, not proof from shell CPU alone:
continue only when model-visible input is materially reduced without a
correctness regression or unacceptable latency increase.

Context reads are phase-specific and bounded: summaries during drafting,
ownership/dependency views during review, changed-document views during
correction, and validator-focused views during final validation. The context
index namespaces the authoritative `SKILL.md`, generated `REVIEWER.md`, and
approved relative references; source or plan hash drift invalidates cached
memory before a read. Per-worker variables and checkpoint state live outside
counted plan deliverables and are isolated by run, revision, and session.

Every phase checkpoint records only the current state, open finding IDs, next
action, changed files, and source/plan hashes. Checkpoints are written
atomically and rejected on identity or hash mismatch. Helper output is quiet
by default, bounded retries return corrected usage, and size budgets warn or
fail without replacing required evidence with prose.

### 4.1.1 Dynamic scope additions are plan mutations

When execution discovers new implementation, verification, risk, or handoff
scope, record the discovery in working context first, then convert it into
durable plan state before treating it as in scope. A durable addition requires
an owning goal or existing goal step, a work-unit inventory row with explicit
dependencies, a step with a concrete acceptance contract, a testing companion
when behavior is verifiable, progress rows, and a handoff/evidence requirement.
Run the plan validator after the mutation and ensure the adversarial-review
artifact explicitly covers the new unit. A note, TODO, status paragraph, or
journey entry alone is not a valid plan addition and must not be marked
complete or used as a release dependency.

