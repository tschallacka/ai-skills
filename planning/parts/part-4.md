<!-- MODE: PROD -->
> Generated from `skill-source.txt` by `scripts/generate-skill-docs.sh` — do not edit.
> Part 4 of 4 — resume and update the plan.
>
> Before treating this part as read: find the line below matching
> `<!-- SKILL-LOAD-PROOF part=part-4 token=... -->` — its position moves on every
> regeneration — and run `planning/scripts/verify-skill-load.sh --part part-4
> --token <the-token-you-found>` before continuing. Naming a token is not
> enough; the command must succeed. If it refuses, you have not finished
> reading this part.

## 4. Resume and update the plan

Run `validate-plan.sh` MUST be re-run at the start of each resumed session
before executing steps, after any plan edit, and before presenting any plan or
plan revision as ready. The validator is the gate that confirms the plan is
still structurally sound after changes.

At the start of every resumed session:

1. Read the plan-level `progress.md`.
2. Continue the goal marked `⏳ in progress`; do not switch goals unless it is
   complete, blocked, or the user changes priority.
3. If no goal is in progress, select the first incomplete goal whose
   prerequisites are complete and mark it `⏳ in progress`.
4. Read that goal's `goal.md`, `progress.md`, and `working-context.md` when it
   exists.
5. Read completed handoffs from prerequisite goals before acting.

During execution:

- Mark the goal and step `⏳ in progress` before starting work.
- Mark a step `✅ completed` only after its implementation and listed checks
  pass.
- Update the relevant progress bar after completion.
- Write the completed handoff before marking the goal complete.
- Keep current facts, the desired outcome, and the next action up to date in
  working context when it exists.
- If execution reveals a scope-changing decision or material uncertainty,
  pause and ask the user before changing the plan.

<!-- REVIEWER_SECTION:START bounded-context -->
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
<!-- REVIEWER_SECTION:END bounded-context -->

### 4.2 Reviewer profile generation

`SKILL.md` is the source of truth for reviewer behavior. Sections marked with
`REVIEWER_SECTION` markers are extracted by
`scripts/generate-reviewer.sh` into `REVIEWER.md`. The generator has an
explicit allowlist of reviewer sections and must fail if a required section is
missing. When reviewer behavior changes, update the marked source section and
the generator allowlist or extraction logic as needed, then run the generator
to refresh `REVIEWER.md` and record the source hash. Do not hand-edit
`REVIEWER.md`.

Use the bundled scripts on Bash or Zsh instead of rebuilding tracker logic or
patching a plan document:

```bash
PLANNING_SKILL_DIR="<installed-planning-skill-directory>"
"$PLANNING_SKILL_DIR/scripts/create-progress.sh" <goal-directory> <goal-name>
"$PLANNING_SKILL_DIR/scripts/create-plan-progress.sh" <plan-directory>
"$PLANNING_SKILL_DIR/scripts/create-plan.sh" <plan-directory> "<plan title>"
"$PLANNING_SKILL_DIR/scripts/plan-root.sh" resolve   # prints the resolved plans root; prompts on first use in a project
"$PLANNING_SKILL_DIR/scripts/create-adversarial-review.sh" <plan-directory>
"$PLANNING_SKILL_DIR/scripts/create-ui-validation.sh" <plan-directory> "<browser target or discovery method>"
"$PLANNING_SKILL_DIR/scripts/add-ui-story.sh" <plan-directory> US-01 "<persona>" "<browser actions>" "<direct interaction>" "<expected result>" W01,W02
"$PLANNING_SKILL_DIR/scripts/configure-ui-story-cache.sh" <plan-directory> US-01 "<starting state>" "<direct UI input>" "<target/value>" "<readiness signal>" "<maximum wait>"
"$PLANNING_SKILL_DIR/scripts/update-ui-story.sh" <plan-directory> US-01 --expected "<corrected expectation>"   # correct a story that turned out to contradict the plan; re-checks the interaction rule against the resulting row
"$PLANNING_SKILL_DIR/scripts/add-goal.sh" <plan-directory> 01-<goal> "<title>" "<outcome>"
"$PLANNING_SKILL_DIR/scripts/add-work-unit.sh" <plan-directory> --id W01 --type <type> --file <file|N/A> --scope <scope> --subscope <subscope|N/A> --change "<change>" --depends-on <dependencies|—> --goal 01-<goal> --step 01-step-<slug>
"$PLANNING_SKILL_DIR/scripts/update-work-unit.sh" <plan-directory> W01 --depends-on "W23,W24"   # change scope/file/type/depends-on/description in place; retargeting lists the verification units that grade it
Ordering note: goals and steps only append (`NN-kebab-case` is enforced, no
renumbering helper exists). Appending plus a prose "execution order differs
from step numbering" note is the sanctioned pattern. Reviewers must not reject
ordering prose that accompanies recorded dependency edges.
"$PLANNING_SKILL_DIR/scripts/add-coverage.sh" <plan-directory> "<outcome or proof>" W01 "<notes>"            # append a coverage row
"$PLANNING_SKILL_DIR/scripts/add-coverage.sh" <plan-directory> "<outcome or proof>" W01,W02 "<notes>" --replace  # amend (collapses duplicate rows for the same outcome)
"$PLANNING_SKILL_DIR/scripts/remove-coverage.sh" <plan-directory> "<outcome or proof>"  # remove an obsolete coverage row (names the work units it carried)
"$PLANNING_SKILL_DIR/scripts/verify-target.sh" <plan-directory> W01 [--repo <root>]   # static reachability check: file exists, layout removes/re-points the block, theme override; a unit with no target, or a render surface with no block name, fails
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --testing-requirement <plan-directory> 01-<goal> <yes|no> "<rationale>"
"$PLANNING_SKILL_DIR/scripts/update-step.sh" <goal-directory> <step-name> in-progress
"$PLANNING_SKILL_DIR/scripts/update-step.sh" <goal-directory> <step-name> completed
"$PLANNING_SKILL_DIR/scripts/update-progress.sh" <goal-directory>
plan-overview --plan-dir <plan-directory> --out <file>   # one-file HTML dashboard
plan-overview --plan-dir <plan-directory> --serve --port <port>   # serve the artifact on loopback
# If no matching prebuilt artifact exists, the installer reports the overview as unavailable.
"$PLANNING_SKILL_DIR/scripts/render-plans-board.sh" [--root <plans-root>] [--out FILE]   # one-file html board across EVERY plan in the root: lifecycle, steps, findings, review, last activity, and a link into each plan's own overview
"$PLANNING_SKILL_DIR/scripts/update-plan-progress.sh" <plan-directory> <goal-name> in-progress
"$PLANNING_SKILL_DIR/scripts/update-plan-progress.sh" <plan-directory> <goal-name> completed
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --title <plan-directory> plan "<title>"
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --description-section <plan-directory> affected-areas -p 6.1: "<first paragraph>" -p 6.2: "<second paragraph>"
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --description-paragraph <plan-directory> 6.1 "<replacement>"
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --table-paragraph <plan-directory> plan 6.1 3 '"Header","Value","Status"\n"Item","He said ""go""","ready"'
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --insert-after <plan-directory> plan 6.1 "<new paragraph>"
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --delete-paragraph <plan-directory> step:01-g/01-step-a 5.2   # delete one paragraph; later labels in the section renumber
"$PLANNING_SKILL_DIR/scripts/update-work-unit.sh" <plan-directory> W88 --scope "RequestEmployeeSet::forCustomer()"   # --scope is the flag form of the scope positional
"$PLANNING_SKILL_DIR/scripts/update-work-unit.sh" <plan-directory> W88 --type source --description "<new intended change>"   # amend type/description in place
"$PLANNING_SKILL_DIR/scripts/create-adversarial-review.sh" <plan-directory>
"$PLANNING_SKILL_DIR/scripts/update-adversarial-review.sh" <plan-directory> --file review.csv      # rewrite the Findings table from a CSV file
"$PLANNING_SKILL_DIR/scripts/update-adversarial-review.sh" <plan-directory> --cycle 7              # archive the prior Findings table under Cycle 7; refused (73) if Cycle 7 already holds other findings
"$PLANNING_SKILL_DIR/scripts/mint-fix-keys.sh" <plan-directory>                                     # (re)derive per-(finding,work-unit) fix keys into fix-keys.json
"$PLANNING_SKILL_DIR/scripts/add-fix-claim.sh" <plan-directory> --finding <AR-NN> --work-unit <WNN> --key <hex>   # record one fix-key claim in fixes.md
"$PLANNING_SKILL_DIR/scripts/verify-fix-keys.sh" <plan-directory> [--claimed-by <session>]          # verify fixes.md claims against fix-keys.json; self-certification fails
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --field <plan-directory> plan 'UI affected' no
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --decomposition-review <plan-directory> completed
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --review-status <plan-directory> approved
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" get <plan-directory> unit:W01 json
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" summary <plan-directory> markdown
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" blast-radius <plan-directory> W01 markdown
"$PLANNING_SKILL_DIR/scripts/create-step-testing.sh" <goal-directory> <step-name> "<instructions>"
"$PLANNING_SKILL_DIR/scripts/create-step-testing.sh" <goal-directory> <step-name> "<instructions>" --overwrite   # replace a companion; input is validated before any file is touched
"$PLANNING_SKILL_DIR/scripts/create-step-testing.sh" <goal-directory> <step-name> "<instructions>" --browser "<instructions>" --backend "<instructions>" --manual "<instructions>"   # the other verification sections; each is optional, and a companion can only be given a section at creation
 "$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" -ss <plan-directory> <goal>/<step>-testing automated-tests -p 2.1: "<first paragraph>" -p 2.2: "<second paragraph>"
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" -sp <plan-directory> <goal>/<step>-testing 2.1 "<replacement>"   # the -testing companion is a writable surface with its own section ids
```
**Document-id prefix rule for `update-plan-content.sh`.** The short flag forms
`-sp/-ss/-gp/-gs/-rp/-rs` take the document id **bare** (`<goal>/<step>` or
`<goal>` or `review`) — the script prepends the `step:`/`goal:`/`review:`
prefix itself. The long forms that replace a whole document (`--paragraph`,
`--table-paragraph`, `--delete-paragraph`, `--field`, `--title`) take the full
prefixed id (`step:<goal>/<step>`); do not add the prefix when the flag is a
short `-s*`/`-g*`/`-r*` form, or you get a doubled `step:step:…` id.
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" find <plan-directory> '<old phrase>' --in all
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" find <plan-directory> '<phrase>' --document step:<goal>/<step>-testing   # verify wording at the surface a finding named
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" find <plan-directory> '<phrase>' --full   # no excerpt truncation
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" get <plan-directory> inventory   # full work-unit inventory
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" diff <plan-directory> <git-ref>   # walks up to the enclosing repo, scoped to the plan subdir
"$PLANNING_SKILL_DIR/scripts/cleanup-plans.sh" --list                          # list plans under the root, marking completed
"$PLANNING_SKILL_DIR/scripts/cleanup-plans.sh" <plan-name> ... [--yes]         # remove selected plans (confirms unless --yes); clears the plans-root git history when the last plan is removed
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" find <plan-directory> '<phrase>' --in coverage   # Definition-of-done coverage rows
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" find <plan-directory> '<phrase>' --in stories    # ui-user-stories.md
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" diff <plan-directory> HEAD   # documents and § paragraphs changed since a git ref
```

Document IDs are `plan`, `review`, `goal:<goal>`, `step:<goal>/<step>`,
`unit:<WNN>`, `coverage` (Definition-of-done coverage table), `stories`
(`ui-user-stories.md`), `inventory` (the work-unit inventory), `fixes`
(`fixes.md`), `fix-keys` (`fix-keys.json`), and `approval` (`approval.json`).
`find` scopes cover `plan`, `goals`, `steps`, `units`, `review`, `testing`,
`coverage`, `stories`, `inventory` (alias for `units`), and `all`.

The creation scripts refuse to overwrite existing trackers. The update
scripts change the requested row and recalculate the relevant progress bar.
`plan-content.sh` supports `markdown`, `text`, and `json` output for summaries
and blast radius, plus `path` for a direct document lookup.

### 4.3 Persistent monitor steering

Monitoring an active worker, reviewer, analyzer, or test process is an
execution loop, not a one-shot status query. Treat a status report, partial
artifact list, unchanged poll, or “I’m working” message as intermediate.
Continue bounded polling and issue an explicit next-action steering command
while the process remains active. Before steering, inspect bounded process
state, latest output, expected artifacts, elapsed time, and retry budget.

Stop only on terminal evidence: process exit with a result, accepted/tainted/
rejected archive, validated completion report, or a recorded blocker after the
configured retry budget. Never restart blindly, hide a real error, or report
success because a subprocess emitted a status-only message. Preserve the last
output, process audit, next action, steering/retry count, and terminal reason.
For repeated long checks, use a uniquely named executable helper under `/tmp`
with explicit run arguments and a bounded selector flag (for example `1` for
runner/worker, `2` for reviewers, and `3` for all in-scope processes); reject
unsupported selector values before inspecting processes.

### 4.4 Planning environment contract evolution

The environment manifest is a versioned planning-skill interface, not a
backward-compatibility layer. When a new variable is needed or an existing
variable changes, treat it as a coordinated schema migration: record the
reason and owner in the plan, update the manifest producer, every applicable
consumer, package inventory, focused rejection/fixture tests, and adversarial
review evidence in the same change. Re-run the plan validator and the
installer-manifest check before completion.

Do not preserve old variable names through aliases, adapters, legacy modes, or
inferred defaults. Replace the manifest and its consumers together; a missing,
stale, unknown, or schema-mismatched manifest must fail closed with an
actionable error. Remove superseded variables from the producer, consumer
allowlists, documentation, and tests, and confirm that no published archive
contains the local manifest.

### 4.5 Helper-only plan mutations and bulk execution

Durable changes under `.plans/` must go through the planning helpers or the
canonical `scripts/plan-mutate.sh` dispatcher. This includes creating goals,
steps, testing companions, inventory rows, progress rows, content changes,
review status, decomposition status, and validation state. Direct editor,
patch, redirection, or ad-hoc script writes to plan artifacts are prohibited.

When many helper mutations are needed, prefer one temporary executable batch
script containing only approved helper commands. Run it with strict mode and
bounded arguments, capture its output, remove it after completion, and run the
structural validator once the batch succeeds. Each helper mutation remains
atomic; a batch failure is recorded as incomplete and is never presented as a
completed plan update. Do not use batching to bypass helper validation or hide
an intermediate failure.

### 4.6 replacement package handoff

The replacement package is repository-owned until its closure plan is
approved. Its finite installable boundary is the six-column
`planning/PACKAGE-MANIFEST.tsv`; `planning/PACKAGE-MAP.tsv` is the
source/destination ownership record and the two repository-root brainstorm
inputs are source-only. The package contains the contract, benchmark and
oracle records, fixtures, runner evidence, installer proof, this skill, and
the planning helper scripts listed by that manifest.

The coordinator resume order is: close authority/recovery, transaction/lease,
package/wire, and benchmark/oracle contracts; generate and verify the bounded
runner; then compare the full manifest, run the plan validator, and preserve
the contract-test report. No backward-compatible adapter, legacy mode, or
inferred default is part of the package. The approval gate is the approved
adversarial review plus complete plan validation; a design review is not a
runtime installation.

After approval, install with the repository installer's explicit planning
target command and the exact manifest: `installer install-skill planning
--target TARGET --approval yes`. Before that boundary, use
`installer print-skill-files planning` and
`installer resolve-source planning RELATIVE_PATH` only for inspection. Declined
approval and destination collisions fail before copy or backup; preserve the
target, record the failure, resolve the collision or approval decision, and
resume the same manifest rather than installing a partial package.

### 4.7 A plan from an older skill version is obsolete, not migrated

This skill does no backwards compatibility, and a plan directory is one of its
interfaces. A plan built by an older version of the skill is therefore not
resumed, not repaired, and not migrated: it is marked obsolete, and the
initiative is rebuilt as a new plan in a new plan directory with the current
tools.

**Detect it from the plan's own `.env`.** `plan-env.sh write-plan` records
`PLAN_ENV_SCHEMA_VERSION`, and that is the version signal — nothing else in the
plan carries one. A plan directory is from an older skill version when its
`.env` is missing, when its `PLAN_ENV_SCHEMA_VERSION` differs from the value
the installed `plan-env.sh` writes, or when
`plan-env.sh check <plan-directory>` refuses it with a schema or manifest error
(exit 65). Run that check before reading anything else in a plan
you did not create in this session.

**Mark it with a file, not with prose.** Write `<plan-directory>/OBSOLETE`:

```
obsoleted-at: <YYYY-MM-DD>
obsoleted-because: built by an older planning-skill version
replaced-by: <new plan directory name>
```

`replaced-by:` is mandatory and names the plan that supersedes this one. The
marker is a separate file rather than a key in `.env` or a status line in
`plan-description.md`, because neither of those can carry it: `.env` is a closed
manifest whose key allow-list rejects an unknown key and whose schema check
demands the current version, and the `- Status:` field is already owned by the
review-status gate, which permits only `💤 pending` and `✅ approved`. A file is

<!-- SKILL-LOAD-PROOF part=part-4 token=2c51bf581dc0df91 -->

also visible in a directory listing, which is where somebody about to resume the
wrong plan is looking.

**`validate-plan.sh` refuses an obsolete plan** before its first pass: exit 65,
naming the replacement, instead of a wall of findings about a plan nobody should
be using. Fix the marker, not the plan, if that refusal is unexpected.

**Nothing is deleted, ever.** The obsolete plan directory stays exactly as it
is — its description, inventory, goals, steps, progress trackers, adversarial
review and handoffs are the record of what was decided and why, and the rebuild
reads them as input. Do not remove it, do not empty it, and do not edit its
documents to make it look current.
