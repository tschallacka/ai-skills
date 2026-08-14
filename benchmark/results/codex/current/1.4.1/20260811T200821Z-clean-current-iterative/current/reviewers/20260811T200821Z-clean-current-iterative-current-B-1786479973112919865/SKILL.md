---
name: planning
description: Use when the user requests a durable plan or a multi-step initiative genuinely needs resumable files, ordered goals and steps, verification instructions, progress trackers, or handoff notes. Do not use for small, self-contained changes or temporary in-chat checklists.
---

# Planning

Use this skill to turn an initiative into a directory of Markdown files that
another agent can resume and execute without reconstructing missing context.

Do not use it for a small, self-contained change or a temporary in-chat plan.

When an initiative creates, changes, repairs, or validates any UI component,
page, interaction, visual state, or user-facing flow, read
[`references/ui-user-story-validation.md`](references/ui-user-story-validation.md)
in full before establishing the plan boundary. Its workflow is mandatory for
that plan; it adds browser-driven discovery, a user-story artifact, and a
bug-priority feedback loop.

## Operating rules

- Keep the plan factual, concise, and executable.
- Separate the initiative into non-overlapping goals. Each goal owns one
  meaningful outcome or area of change.
- Make every goal and step self-contained. Include the context needed to act;
  do not require the agent to infer details from unrelated files.
- Record confirmed facts separately from assumptions. Ask the user only when
  an unresolved choice could materially change scope, implementation, risk, or
  verification. Otherwise make a reasonable assumption and record it.
- Do not invent details to make a plan appear complete. Mark unknowns as open
  questions or risks.
- Keep progress accurate. A completed status requires the implementation and
  all applicable verification to have passed.
- Treat decomposition as a design activity, not a formatting activity. Do not
  create goal or step files until the work-unit inventory and ownership map in
  section 2.2 pass their checks.
- Do not hide multiple edits behind a broad verb such as "implement",
  "update", "integrate", or "wire up". Name the concrete file and symbol (or
  file-level scope) that changes.

## Tool discipline and context limits

Planning must stay within the agent's available context budget. When the
environment provides context-limiting tools—such as bounded reads, result
limits, pagination, summarization, compaction, or scoped subagent contexts—use
them while researching and reviewing a plan. Request only the files, symbols,
logs, and output needed for the current work unit; do not load an entire
repository or an unbounded command result when a scoped query is sufficient.

Use the planning skill's bundled shell scripts for creating, reading, and
mutating plan documents, trackers, inventories, reviews, and UI artifacts.
Do not reconstruct their behavior with ad-hoc patches or one-off text
rewrites. If a required helper is missing, add it to the skill before using
the workflow and keep its output validator-compatible.

Use the repository's available code-lookup tools before broad text searches
when discovering implementation files, symbols, callers, dependencies, or
blast radius. This may be an indexed code graph, symbol search, language
server query, IDE index, or another repository-aware lookup facility. Scope
the lookup to the named work unit and follow its pagination or result limits.
Use text search only for literals, non-code documents, configuration values,
or when the repository-aware lookup cannot answer the question.

These rules are agent-generic: use the strongest context limiter, shell
workflow, and code-lookup facility available in the current environment, and
record any unavailable facility as a plan constraint when it affects
discovery or verification.

## Hard planning gates

These rules are mandatory for a new or materially revised plan. A plan is not
ready to execute until it passes the decomposition and validation gates below.

### Atomic work-unit limit

A **work unit** is one independently reviewable change target:

- **Source code:** one file and one function, method, class, component, or
  other named primary symbol in it. A top-level static class, constant,
  initializer, or declaration is also valid when no executable symbol exists.
  Name one optional nested loop, callback, branch, or anonymous function as a
  subscope when that is the actual change target; otherwise use `N/A`.
- **Markup:** one file and one named DOM subtree or template block (for
  example, `#checkout-summary`).
- **Style:** one file and one CSS selector or named style token (for example,
  `.completion-message`).
- **Configuration:** one file and one precisely named key, route, declaration,
  or section.
- **Test:** one test file and one test class or test function.
- **Documentation, migration, fixture, or asset:** one file and one named
  section, migration, fixture, or asset.
- **Verification:** one named command or one bounded browser/API flow. It has
  no implementation file, but it is still its own work unit and step.

One implementation step owns exactly one work unit. It may not include a
second source file, second symbol, second test target, or a catch-all such as
"related callers." Make those separate, ordered steps even when the changes
are mechanically small. Do not use globs, directory names, or "all affected
files" as a target.

An exception is allowed only for an inseparable generated-file update. Record
the generator command and every generated file in the step, set its type to
`generated`, and explain why individual review is impossible. Never use this
exception for ordinary source, configuration, test, or documentation edits.

### Goal size limit

A goal owns one coherent, independently demonstrable outcome and normally
contains **2–10 work units**. A single-work-unit goal is allowed only for a
genuinely standalone documentation, configuration, discovery, or verification
outcome; state the reason in its `goal.md`. A goal with more than 10 work
units is invalid and must be split at the next stable product, contract,
deployability, or ownership boundary. Do not split merely by file type.

Every goal needs its own definition of done that can be demonstrated without
claiming completion of later goals. If it cannot be demonstrated independently,
it is a segment of another goal, not a goal.

## 1. Establish the plan boundary

Before creating plan files, establish enough information to write an
executable plan:

- Desired outcome and definition of done
- In-scope and explicitly out-of-scope behavior
- Affected files, modules, layouts, services, data, or systems
- Constraints, ownership boundaries, conventions, and compatibility concerns
- User-visible behavior and required browser verification, if any
- Backend behavior and required unit or integration verification, if any
- Dependencies on other goals or external systems

Ask focused follow-up questions for material gaps. Do not ask for details that
can be discovered safely from the repository or environment. If the user does
not need to choose between materially different approaches, choose one,
explain the assumption in the plan, and continue.

Do not create implementation files until each goal has a clear owner, boundary,
outcome, and definition of done.

## 2. Create the plan directory

Choose a short, descriptive, kebab-case `<planname>` such as
`checkout-totals-own-page`.

Create the plan under the user-owned `.plans/` root resolved by the helper:

```text
<plans-root>/<planname>/
```

For a normal Unix home directory, this is:

```text
~/.plans/<planname>/
```

Set `PLANS_ROOT` when a different user-owned location is required. Keep the
planning skill installation and durable plan storage separate.

Create it with the bundled command; do not create the directory or its initial
documents with a patch. It creates a canonical `plan-description.md` and an
empty work-unit inventory that the other commands can update safely:

```bash
PLANNING_SKILL_DIR="<installed-planning-skill-directory>"
PLANS_ROOT="${PLANS_ROOT:-$HOME/.plans}"
"$PLANNING_SKILL_DIR/scripts/create-plan.sh" \
  "$PLANS_ROOT/<planname>" "<plan title>"
```

Use the flagged `update-plan-content.sh` commands for narrative edits; the
helpers enforce paragraph numbering, spacing, sequencing, and safe content.

### 2.1 Write the plan description

Create `<planname>/plan-description.md`. It must contain:

- **Current state:** confirmed facts, available assets, and relevant prior
  work
- **Desired outcome:** the initiative's definition of done
- **Approach:** agreed sequence and major implementation decisions
- **Scope:** included and explicitly excluded behavior
- **Affected areas:** files, modules, layouts, services, data, and systems
- **Constraints and decisions:** permissions, ownership, conventions, and
  user choices
- **Risks and open questions:** only items that could affect execution

Use one clear section per topic. Do not duplicate goal-specific implementation
details here; put them in the owning goal.

The canonical plan description has replaceable `title`, `current-state`,
`desired-outcome`, `approach`, `scope`, `affected-areas`,
`constraints-and-decisions`, and `risks-and-open-questions` sections. Replace
their narrative content through flagged `update-plan-content.sh` targets; use
`--title` for the document title and `--field` for structured values such as
`UI affected`.

### 2.2 Compile the work-unit inventory before choosing goals

Create `<planname>/work-unit-inventory.md` before creating goal directories.
This is the plan's source of truth for scope and step ownership. It forces the
agent to reason from concrete changes upward rather than guessing a few broad
goals and writing generic steps beneath them.

For a plan created with `create-plan.sh`, add facts to its empty inventory with
the bundled commands instead of patching table rows:

```bash
PLANNING_SKILL_DIR="<installed-planning-skill-directory>"
"$PLANNING_SKILL_DIR/scripts/add-coverage.sh" <plan-directory> \
  "<required outcome or proof>" W01,W02 "<why these units cover it>"
"$PLANNING_SKILL_DIR/scripts/add-work-unit.sh" <plan-directory> W01 source \
  path/to/file 'Class::method()' N/A "<one concrete change>" '—' \
  01-<goal> 01-step-<slug>
```

`add-work-unit.sh` creates both the inventory row and its matching atomic step
file, so their ownership fields cannot drift. Add the goal first. Do not
continue an older plan whose documents predate the current skill contract. If
an update is requested for such a plan, stop and ask the user to rewrite it
with the current helpers before proceeding; do not retrofit it in place.

Work through this sequence in order. Do not skip a question because the answer
seems obvious:

1. **Expand the requested outcome.** List each user-visible behavior,
   contract, data change, operational concern, and required verification that
   must be true at definition of done.
2. **Discover the change surface.** Inspect the repository, design, and
   environment. List every known file and symbol to create or change. When an
   exact file or symbol is not yet knowable, add a bounded discovery work unit
   first; never place `TBD` into an implementation step.
3. **Atomize.** Turn each target into one work unit under the atomic limit.
   Split a file that has multiple independently changing functions or classes
   into one work unit per symbol. Split HTML markup, CSS selectors, and
   executable functions into separate work units even when they are in the
   same file.
4. **Add proof.** Add separate work units for each test target and each
   required browser, API, command, migration, or manual verification flow.
5. **Order dependencies.** For every unit, identify what it consumes and what
   it enables. Do not rely on directory order to imply a dependency.
6. **Form goal candidates.** Group only adjacent, mutually necessary work
   units that produce one demonstrable outcome. Apply the goal size limit;
   split candidates at the first stable boundary.
7. **Assign one step per unit.** Give every unit exactly one owning goal and
   exactly one numbered implementation or verification step. There must be no
   unowned, multiply owned, or bundled units.
8. **Challenge the result.** For every goal, ask: “Could an executor finish
   this without editing a file or symbol not named below?” For every step ask:
   “Could a reviewer approve this without reviewing a second target?” Split
   the goal or step whenever either answer is no.

Let `create-plan.sh`, `add-coverage.sh`, `add-work-unit.sh`, and
`decomposition-review` create and update the inventory. They enforce the table
columns, stable IDs, ownership, and review checklist. Mark the checklist
complete only after checking the resulting rows.

### 2.3 Decompose the initiative into goals

Divide the initiative into a small, ordered set of cohesive goals. Order goals
by dependency when that matters. Do not split a user-visible outcome merely by
technical layer or file type.

Create each goal with the helper; it creates the directory, step directory,
and all mandatory sections in canonical order:

```bash
"$PLANNING_SKILL_DIR/scripts/add-goal.sh" <plan-directory> 01-<goal-slug> \
  "<goal title>" "<outcome and definition of done>"
```

Each `goal.md` must be executable on its own and contain:

- Current state and relevant prior-goal handoffs
- The goal's outcome and definition of done
- Why the goal is needed and how it contributes to the initiative
- In-scope and explicitly out-of-scope behavior
- Affected files, systems, data, and interfaces
- Dependencies and precise handoffs with other goals
- Implementation approach, risks, and relevant edge cases
- **Owned work units:** the exact IDs from `work-unit-inventory.md`, with a
  concise explanation of their shared outcome
- **Testing requirement:** a table declaring `yes` or `no` and a rationale;
  research-only or genuinely untestable goals may declare `no`
- **Goal-size exception:** required only when the goal has one work unit; cite
  the applicable hard-gate exception

Keep ownership clear. Document a shared contract once in
`plan-description.md`; reference it from goals instead of copying it. When a
goal depends on an earlier goal, read that goal's completed handoff before
composing or executing the dependent goal.

Use `no` only for a genuinely untestable or research-only goal and explain why.
When the table says `yes`, the goal must own at least one `test` or
`verification` work unit. When a goal owns a test or verification work unit,
the table must say `yes`.

### 2.4 Add working context when needed

Create `<goalname>/working-context.md` only when execution produces useful,
goal-specific facts that do not belong in `goal.md`. Keep it concise and
factual. Examples include test accounts, fixture IDs, routes, discovered file
locations, environment quirks, limited commands, and user decisions.

Update this file as facts are confirmed. Do not rewrite the original goal to
include runtime discoveries. When the goal is complete, the `Handoff` section
is required and must state what later goals can rely on.

### 2.5 Break each goal into atomic steps

Create one ordered step for each assigned work unit. Do not create a step
before its work unit exists in the inventory. A goal with N work units must
contain exactly N implementation/verification step files (plus any applicable
testing companion files).

Use `add-work-unit.sh` from section 2.2 to create the step together with its
inventory row. It creates the mandatory headings, ownership fields, atomicity
checks, and numbered narrative paragraphs. Update the objective, instructions,
acceptance criteria, and handoff with `update-plan-content.sh`; do not patch a
step file directly.

When a goal declares `Test required: yes`, the step writer prints a reminder to
continue with the test/proof step. If a testing companion already exists, the
reminder directs the agent to review it for accuracy and completeness after
the step changes.

Each step file must contain:

- The owning goal and this step's objective
- The **single work-unit ID** it owns
- Work-unit type (`source`, `test`, `config`, `docs`, `data`, `generated`,
  `markup`, `style`, `discovery`, or `verification`)
- Exactly one file, primary symbol or file scope, and subscope, copied from
  the inventory; use `File: N/A` only for a verification unit and `Subscope:
  N/A` when no nested target is changed
- Directly executable implementation instructions
- Acceptance criteria for this step
- Any handoff needed by a later step
- An atomicity check confirming that no other change target is included

The only files permitted in a step are the single target and an explicitly
listed generated output under the `generated` exception. A source step does
not also “add tests”; create its test work unit and step separately. A test
step does not also change production code. A verification step does not also
make fixes.

After decomposing a goal, check that its inventory rows and step files are a
one-to-one mapping. If a material uncertainty remains, create a bounded
discovery work unit or resolve it before continuing.

<!-- REVIEWER_SECTION:START mandatory-review -->
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

<!-- REVIEWER_SECTION:END mandatory-review -->

### 3.1 Add verification instructions

For every step with verifiable behavior in a goal whose testing requirement is
`yes`, create a companion file with the same number and slug:

```text
<goalname>/steps/01-step-<short-slug>-testing.md
```

Omit the file when the goal's testing requirement is `no` or when there is
genuinely nothing to verify, such as a pure documentation step. If a step is
updated through the CLI and its companion already exists, review that
companion for accuracy and completeness before continuing.
The validator enforces companions for every non-documentation step in a goal
marked `yes`.

Include only the relevant sections:

- **Browser verification:** exact navigation and actions, the expected result,
  and explicit pass/fail criteria. Use the browser tools or browser-testing
  skill available in the environment.
- **Backend verification:** concrete commands and inputs to exercise the
  behavior against the running system, plus expected output.
- **Automated tests:** required unit or integration tests, their locations,
  commands, and relevant project testing conventions.

A step may require multiple verification methods. Do not mark it complete until
all listed checks have actually passed.

### 3.2 Validate, then create progress trackers

Run the validator before creating trackers or presenting the plan as ready:

```bash
PLANNING_SKILL_DIR="<installed-planning-skill-directory>"
"$PLANNING_SKILL_DIR/scripts/validate-plan.sh" <plan-directory>
```

Do not waive validation failures. Correct the inventory, goal boundary, or
step files and run it again. The validator checks the structural guarantees;
the decomposition review remains required for semantic completeness.

For a goal marked `Test required: yes`, every implementation, markup, style,
configuration, data, or generated work unit must have a downstream `test` or
`verification` unit in the dependency graph. A goal marked `no` may omit that
proof when its rationale records why testing is not meaningful or possible.
Do not use `no` to avoid testing observable behavior.

Create plan and goal progress trackers with the bundled creation helpers;
they enforce the table shape and initialize every item as `💤 incomplete`.

Use these statuses consistently:

- `💤 incomplete` — not started
- `⏳ in progress` — currently being worked on
- `✅ completed` — implementation and all applicable verification passed

Progress percentages count completed items equally. A goal is complete only
when all its steps and applicable verification are complete. The initiative is
complete only when all goals are complete.

## 4. Resume and update the plan

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
"$PLANNING_SKILL_DIR/scripts/create-adversarial-review.sh" <plan-directory>
"$PLANNING_SKILL_DIR/scripts/create-ui-validation.sh" <plan-directory> "<browser target or discovery method>"
"$PLANNING_SKILL_DIR/scripts/add-ui-story.sh" <plan-directory> US-01 "<persona>" "<browser actions>" "<direct interaction>" "<expected result>" W01,W02
"$PLANNING_SKILL_DIR/scripts/configure-ui-story-cache.sh" <plan-directory> US-01 "<starting state>" "<direct UI input>" "<target/value>" "<readiness signal>" "<maximum wait>"
"$PLANNING_SKILL_DIR/scripts/add-goal.sh" <plan-directory> 01-<goal> "<title>" "<outcome>"
"$PLANNING_SKILL_DIR/scripts/add-work-unit.sh" <plan-directory> W01 <type> <file|N/A> <scope> <subscope|N/A> "<change>" <dependencies|—> 01-<goal> 01-step-<slug>
"$PLANNING_SKILL_DIR/scripts/add-coverage.sh" <plan-directory> "<outcome or proof>" W01 "<notes>"
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --testing-requirement <plan-directory> 01-<goal> <yes|no> "<rationale>"
"$PLANNING_SKILL_DIR/scripts/update-step.sh" <goal-directory> <step-name> in-progress
"$PLANNING_SKILL_DIR/scripts/update-step.sh" <goal-directory> <step-name> completed
"$PLANNING_SKILL_DIR/scripts/update-progress.sh" <goal-directory>
"$PLANNING_SKILL_DIR/scripts/update-plan-progress.sh" <plan-directory> <goal-name> in-progress
"$PLANNING_SKILL_DIR/scripts/update-plan-progress.sh" <plan-directory> <goal-name> completed
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --title <plan-directory> plan "<title>"
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --description-section <plan-directory> affected-areas -p 6.1: "<first paragraph>" -p 6.2: "<second paragraph>"
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --description-paragraph <plan-directory> 6.1 "<replacement>"
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --table-paragraph <plan-directory> plan 6.1 3 '"Header","Value","Status"\n"Item","He said ""go""","ready"'
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --insert-after <plan-directory> plan 6.1 "<new paragraph>"
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --field <plan-directory> plan 'UI affected' no
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --decomposition-review <plan-directory> completed
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --review-status <plan-directory> approved
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" get <plan-directory> unit:W01 json
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" summary <plan-directory> markdown
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" blast-radius <plan-directory> W01 markdown
```

The creation scripts refuse to overwrite existing trackers. The update
scripts change the requested row and recalculate the relevant progress bar.
`plan-content.sh` supports `markdown`, `text`, and `json` output for summaries
and blast radius, plus `path` for a direct document lookup. Document IDs are
`plan`, `review`, `goal:<goal>`, `step:<goal>/<step>`, and `unit:<WNN>`.

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

### 4.2 v27 replacement package handoff

The v27 replacement package is repository-owned until its closure plan is
approved. Its finite installable boundary is the six-column
`planning/V27-PACKAGE-MANIFEST.txt`; `planning/V27-PACKAGE-MAP.tsv` is the
source/destination ownership record and the two repository-root v27 brainstorm
inputs are source-only. The package contains the v27 contract, benchmark and
oracle records, fixtures, runner evidence, installer proof, this skill, and
the 28 planning helper scripts listed by that manifest.

The coordinator resume order is: close authority/recovery, transaction/lease,
package/wire, and benchmark/oracle contracts; generate and verify the bounded
runner; then compare the full manifest, run the plan validator, and preserve
the contract-test report. No backward-compatible adapter, legacy mode, or
inferred default is part of the package. The approval gate is the approved
adversarial review plus complete plan validation; a design review is not a
runtime installation.

After approval, install with the repository installer's explicit planning
target command and the exact manifest: `bash install.sh --install-skill
planning --target TARGET --approval yes`. Before that boundary, use
`--print-skill-files planning --format=tsv` and
`--resolve-source planning RELATIVE_PATH` only for inspection. Declined
approval and destination collisions fail before copy or backup; preserve the
target, record the failure, resolve the collision or approval decision, and
resume the same manifest rather than installing a partial package.
