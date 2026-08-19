# Reviewer contract

> Generated from `SKILL.md` by `scripts/generate-reviewer.sh`.
> Reviewer profile contract: `1.4.2`
> Source SHA-256: `d5a35bbd251c4efb954d0014005a747f28275fb774f36639592f2f48c165d7a7`

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

**Environment facts go to a reviewer from the plan's own working context, not
from prose in the brief.** A reviewer brief that hard-codes a schema name,
socket, or active theme and gets it wrong carries a false fact into every
parallel review session. Prefer putting such facts in `working-context.md`
(verified) and telling the reviewer to read it from there; when a fact cannot
be verified, mark it as an assumption rather than asserting it.

The fresh adversary must be **bounded-read locked**. Hand it the exact reader
command, plan directory, and supported entry ids/views. Its starting prompt
must include verbatim:

"Read plan files and artifacts ONLY through the gated reader:
  "<PLANNING_SKILL_DIR>/scripts/plan-context.sh" read --plan-dir <PLAN_DIR> --document ID
  "<PLANNING_SKILL_DIR>/scripts/plan-context.sh" read --plan-dir <PLAN_DIR> --unit WNN
Valid --document IDs: plan, inventory, progress, adversarial-review,
goal:<goal id>, step:<goal>/<step>. Each read returns one PAGE, not the
document. inventory and adversarial-review default to the whole-document `full`
view; every other id defaults to `summary`, and `--view full` is available for
any of them. A page that withheld records reports next_token — pass it back as
--token and keep going until no next_token comes back. You have NOT read a
document until a page returns without one; treat a page you stopped early on as
an unread document and say so. Plan-read bytes are capped at
the per-role budget when ROLE_ID is set (the gate lowers --max-bytes to it) —
do not rely on --max-bytes above that cap; page instead. Never load a whole
plan file, an entire plan directory, or the `.plans/` tree wholesale. A
wholesale file read of a plan artifact is a context-overflow violation. If the
gate cannot give you something, report it as a limitation — do not bypass it."

The fresh adversary assumes the **chris placeholder persona** (oriented scout):
spawn it with `ROLE_ID=chris`, have it load its scoped role docs and voice via
`"<PLANNING_SKILL_DIR>/scripts/role-context.sh" chris` (which injects its
stance preamble), and require it to state its persona id in the returned
findings. The adversary forms its own findings from the bounded-read gate and
its scoped role docs; it never receives the planning agent's conclusions. A
spawn that cannot resolve ROLE_ID=chris fails closed (the reader refuses) and
must be respawned with a valid identity.

**Scope note: the persona, capsule, and Reviewer A/B machinery describe the
review harness.** When the role-context/capsule tooling (`role-context.sh`, a
capsule workspace) is present in the environment, use it as described. When it
is not — an ordinary plan in a generic environment — the requirement reduces
to: use a **fresh secondary agent with a new session and no prior conclusions**
(bounded-read locked and skill-locked as above) to produce the adversarial
review; the persona, capsule manifest, and two-reviewer A/B split are
harness-specific and OPTIONAL.

Do not hand the adversary a command that dumps a plan file or directory in
full. Require the adversary's returned findings to state that all plan reads
went through the gate and to list any wholesale read it performed, so a
violation is visible (soft audit).

Do not allow the planning agent to approve its own review. **Re-run a fresh
reviewer whenever a revision changes scope, ownership, dependencies, or
acceptance criteria (a material change), or when a bug is discovered** — see
"Execution order is mandatory" below for the exact cadence.

The reviewer protocol is version `1.4.2`. Fresh-review mode remains the
default. Iterative mode is opt-in and must be bounded by a maximum of three
verification passes per reviewer and three fresh-review cycles per benchmark.
Reviewer records use `review_cycle`, `reviewer_session`, `finding_owner`,
`verification_pass`, `closed_findings`, `reviewer_handoff`, and
`review_mode`. Reviewer A MAY close only findings it owns and MUST NOT issue
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

**A finding is a hypothesis, not a work order.** Verify a finding's factual
claims against the codebase before acting. Findings are evidence-backed
hypotheses: some are wrong, some are narrower than stated, and some name the
right defect at the wrong location. Acting on a finding without verifying it
produces a second defect on top of the first.

**A review that finds nothing blocking is a valid and valuable result.** Say so
plainly. Prior cycles finding real defects do not obligate this one to.

**Resolving a finding.** A finding names a symptom, not the full extent of the
defect. A work unit's behaviour is defined across **seven surfaces**, and a
finding cites exactly one. Before recording a resolution, sweep all seven:

1. **Instructions** — step `.md` §5.x; the implementer builds what is written
   here, so a fix that lands only here has not reached the other six.
2. **Acceptance criteria** — step `.md` §6.x; an unfixed criterion is worse
   than an unfixed pair, because the gate now certifies the defect. Move the
   criterion with the instruction, in the same edit.
3. **Inventory description** — the `work-unit-inventory.md` row; the scheduler
   reads the old intent here, and no prose edit reaches a table row. Check the
   row's description, target, type, and dependencies explicitly.
4. **Change target / file / scope** — the same row plus the step header; if it
   lags, work lands on the wrong file.
5. **Goal owned-unit roster** — `goal.md` §9.1; a unit omitted here is
   effectively unowned.
6. **Dependency edges** — the row's Depends-on column; a verification runs
   before what it verifies when the edge lags.
7. **Testing companion** — `<step>-testing.md`; the executor runs the old
   procedure when this lags. It is a real surface with its own writer:
   `update-plan-content.sh -ss <plan> <goal>/<step>-testing automated-tests
   -p 2.1: …` (and `create-step-testing.sh --overwrite` to replace it). Read
   the companion first, not last — on plans with verification-heavy goals it is
   where execution actually happens.

Two further artifacts became readable in later cycles and are worth sweeping
when a finding mentions them: the **Definition-of-done coverage table** and
**`ui-user-stories.md`** (call them 8 and 9 for an exhaustive checklist) — a
finding is not closed until every surface that mentions the behaviour says the
same thing.

Mechanically: search the whole plan for the **old** wording, not the new
(`plan-content.sh find <plan> "<old phrase>" --in all`), fix every site it
appears in including sibling units and goal documents, check the inventory row
(3), move the acceptance criteria with the instruction (2), confirm the unit is
named in its goal roster (5), and re-run the search to confirm the only
remaining hits are deliberate references to the corrected history.

A resolution recorded without the sweep is a claim, not a fix. The
verification-one-unit-away variant is the hardest: a unit may be correct across
all seven surfaces while the verification unit that grades it still checks the
old behaviour. Whenever a unit's change target, scope, or behaviour changes,
re-read the verification unit that grades it (`--propagation` surfaces which
verification units name it).

**Prose ordering is not a plan addition.** Goals and steps append only
(`NN-kebab-case` is enforced; there is no renumbering helper). A prose note that
"execution order differs from step numbering" is sanctioned documentation when
the dependency edges are also recorded; it is not a substitute for those edges.
Reviewers reject ordering prose used as a substitute for recorded dependencies.

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
The `-p N.N:` forms auto-create only when the section's labels are contiguous
`1..max` with no trailing unlabeled content — a section with gaps or
unlabeled paragraphs must be re-authored (e.g. re-run
`create-step-testing.sh --overwrite` so every paragraph gets its `§ N.x`
label) instead of silently appending.

Run the validator again after revisions and reopen the adversarial review when
the change affects scope, ownership, dependencies, or acceptance criteria.

Reviewer duties for reviewer-gated fix keys: when writing or updating the
`## Findings` table, keep the mandatory-with-blank-allowed `Work unit` column
(see 3.1 below) and let `update-adversarial-review.sh` re-mint the derived fix
keys. When recording which key a fix used, write one claim line per
(finding, work unit) into `fixes.md` (`finding_id`, `work_unit`, `key`,
tab-separated). The approval gate auto-verifies `fixes.md` claims against
`fix-keys.json` before flipping the review status to `approved`.

**Reviewers may write `adversarial-review-incoming.md`** (the one plan file a
reviewer may write, so findings survive the coordinator's context).
`update-adversarial-review.sh` consumes it as its findings source and removes
it after the table is rewritten. A reviewer writes its Findings CSV rows there;
the coordinator runs `update-adversarial-review.sh <plan>` (no `--file`) to
land them.

**Reviewers mint fix keys; fixers claim them. Never the same session.** A
reviewer mints the keys by publishing its findings; the fixer claims the keys
in `fixes.md`. Minting and claiming in the same session is self-certification:
`verify-fix-keys.sh --claimed-by <session>` FAILS when the claiming session is
the session recorded as `minted_by`, and the approval gate always passes a
claiming session, so a self-certified fix set cannot be approved. If a fixer
must mint its own keys to record a finding the reviewer
missed, surface it as an open finding for a fresh review rather than resolving
it on its own authority.


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

**A read returns one page, and the budgets bound the page, not the document.**
Whenever a page withholds records it reports `next_token`; pass that value back
as `--token` and repeat until a page comes back without one. The token carries
the document's hash and view, so a token replayed after the document changed is
refused (exit 65) rather than resuming into shifted records — re-read from page
one after such a refusal. Raising `--max-records`/`--max-bytes` enlarges the
page up to the per-role byte cap; past that cap the only way to see the rest of
a document is to page.

**All plan reads go through the gated readers** (see
[`references/plan-read-contract.md`](references/plan-read-contract.md)). Both
the planning agent and every fresh subagent (adversarial review, reviewer) MUST
read plan documents via the bundled gated readers (`plan-context.sh`), never by
loading a whole plan file or plan directory. Wholesale `Read`/`cat`/`head`/`tail`
of a plan artifact is a context-overflow violation, because the gated readers
strip plan metadata and metadata-bearing front-matter that is not needed to act.
Subagents receive this instruction in their starting prompt (see section 3).
Plans use `PLANS_ROOT` when set, otherwise the home directory plus `.plans`,
with `USERPROFILE` and `HOMEDRIVE`/`HOMEPATH` support for Windows-compatible
Bash.

The Phase 1 command contract is fixed: `init` takes only `--plan-dir`; `read`
takes exactly one `--document` or `--unit` plus optional `--view`, `--token`,
`--format`, `--max-bytes`, `--max-records`, and `--read-only`; `check` takes
exactly one of `--entry`, `--changed`, or `--all`; and `refresh` takes exactly
one of `--entry` or `--stale`. Defaults are `text`, 32768 bytes, and 128
records; the default view is `full` for `inventory` and `adversarial-review`
and `summary` for every other id. `--all` audits without registering entries.
Global IDs, Git history,
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

