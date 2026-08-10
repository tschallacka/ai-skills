# Reviewer contract

> Generated from `SKILL.md` by `scripts/generate-reviewer.sh`.
> Reviewer profile contract: `1.4.2`
> Source SHA-256: `a035d7738f890e561a7de3b18a79ae05346894716f478ca1c0710d0c3e0bfa2c`

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

Do not allow the planning agent to approve its own review. Re-run the review
after a material scope change or a discovered bug.

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

