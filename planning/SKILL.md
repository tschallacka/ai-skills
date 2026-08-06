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
  section 4 pass their checks.
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

Create the plan under the `plans/` directory of the installed planning skill:

```text
<planning-skill-dir>/plans/<planname>/
```

For the common Claude Code installation, this is:

```text
~/.claude/skills/planning/plans/<planname>/
```

Use the actual installation directory when the skill is installed elsewhere.

Create it with the bundled command; do not create the directory or its initial
documents with a patch. It creates a canonical `plan-description.md` and an
empty work-unit inventory that the other commands can update safely:

```bash
PLANNING_SKILL_DIR="<installed-planning-skill-directory>"
"$PLANNING_SKILL_DIR/scripts/create-plan.sh" \
  "$PLANNING_SKILL_DIR/plans/<planname>" "<plan title>"
```

All narrative plan, goal, step, and review paragraphs use a stable label such
as `§ 2.1`, followed by the paragraph on the next line. A blank line precedes
every label. Never hand-edit or renumber those labels: use the document update
commands in section 10 so a replacement always targets one unambiguous
paragraph and preserves the required Markdown style.

Paragraph content is supplied as explicit arguments, not as an unstructured
body. The shell requires `-p N.N: content` (repeat `-p` for each paragraph),
requires the section prefix and sequential numbering to match, and immediately
rejects reserved `§` or `-p` tokens inside paragraph text.

## 3. Write the plan description

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
their narrative content through `update-plan-content.sh section`; use `title`
for the document title and `field` for structured values such as `UI affected`.

## 4. Compile the work-unit inventory before choosing goals

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
file, so their ownership fields cannot drift. Add the goal first. The older
`create-work-unit-inventory.sh` remains available only when adopting an
already-created plan directory.

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

Use this exact structure:

```markdown
# Work-unit inventory: <planname>

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|
| <outcome> | W01, W02 | <why these units cover it> |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|
| W01 | source | `path/to/file` | `Class::method()` | `N/A` | <one concrete change> | — | 01-<goal> | 01-step-<slug> |
| W02 | test | `path/to/test` | `test_method` | `N/A` | <one assertion target> | W01 | 01-<goal> | 02-step-<slug> |
| W03 | verification | `N/A` | `<one command or flow>` | `N/A` | <proof to run> | W01, W02 | 01-<goal> | 03-step-<slug> |

## Decomposition review

- [ ] Every definition-of-done item maps to one or more work units.
- [ ] Every known affected file and changing symbol has its own work unit.
- [ ] Every work unit has exactly one goal and one step.
- [ ] Each goal has 2–10 work units, or records an allowed exception.
- [ ] Each step has exactly one work unit and no unnamed incidental edits.
- [ ] Dependencies form an executable order with no cycle.
```

Replace every checkbox with `[x]` only after verifying it against the table.
Use stable IDs (`W01`, `W02`, …); steps must refer to these IDs verbatim.

## 5. Decompose the initiative into goals

Divide the initiative into a small, ordered set of cohesive goals. Order goals
by dependency when that matters. Do not split a user-visible outcome merely by
technical layer or file type.

Create each goal with the helper; it creates the directory, step directory,
and all mandatory sections in canonical order:

```bash
"$PLANNING_SKILL_DIR/scripts/add-goal.sh" <plan-directory> 01-<goal-slug> \
  "<goal title>" "<outcome and definition of done>"
```

The plan must have this structure:

```text
<planname>/
├── plan-description.md
├── work-unit-inventory.md
├── progress.md
├── 01-<goal-slug>/
│   ├── goal.md
│   ├── progress.md
│   ├── working-context.md       # optional until useful
│   └── steps/
│       ├── 01-step-<short-slug>.md
│       └── 01-step-<short-slug>-testing.md
└── 02-<goal-slug>/
    └── ...
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
- **Goal-size exception:** required only when the goal has one work unit; cite
  the applicable hard-gate exception

Keep ownership clear. Document a shared contract once in
`plan-description.md`; reference it from goals instead of copying it. When a
goal depends on an earlier goal, read that goal's completed handoff before
composing or executing the dependent goal.

## 6. Add working context when needed

Create `<goalname>/working-context.md` only when execution produces useful,
goal-specific facts that do not belong in `goal.md`. Keep it concise and
factual. Examples include test accounts, fixture IDs, routes, discovered file
locations, environment quirks, limited commands, and user decisions.

Use this structure when the sections apply:

```markdown
# Working context: <goalname>

## Current state
- <confirmed facts, assets, IDs, commands, or prior-goal handoff>

## Next action
- <next agreed action>

## Handoff
- Outcome: <what is now present>
- Files/data/routes/assets: <what later work can rely on>
- Verification: <checks that passed>
- Caveats: <remaining constraints or risks>
```

Update this file as facts are confirmed. Do not rewrite the original goal to
include runtime discoveries. When the goal is complete, the `Handoff` section
is required and must state what later goals can rely on.

## 7. Break each goal into atomic steps

Create one ordered step for each assigned work unit. Do not create a step
before its work unit exists in the inventory. A goal with N work units must
contain exactly N implementation/verification step files (plus their optional
testing companion files).

Use `add-work-unit.sh` from section 4 to create the step together with its
inventory row. It creates the mandatory headings, ownership fields, atomicity
checks, and numbered narrative paragraphs. Update the objective, instructions,
acceptance criteria, and handoff with `update-plan-content.sh`; do not patch a
step file directly.

Store steps in `<goalname>/steps/` using two-digit, zero-padded prefixes:

```text
01-step-<short-slug>.md
02-step-<short-slug>.md
```

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

Use this exact step skeleton:

```markdown
# <step name>

## Ownership
- Goal: `01-<goal-slug>`
- Work unit: `W01`
- Type: `source`

## Change target
- File: `path/to/file`
- Primary symbol or file scope: `Class::method()`
- Subscope: `N/A`

## Objective
<one outcome for this target>

## Instructions
1. <direct action on this one target>

## Acceptance criteria
- <observable result for this target>

## Handoff
- <what the next named work unit can rely on>

## Atomicity check
- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
```

The only files permitted in a step are the single target and an explicitly
listed generated output under the `generated` exception. A source step does
not also “add tests”; create its test work unit and step separately. A test
step does not also change production code. A verification step does not also
make fixes.

After decomposing a goal, check that its inventory rows and step files are a
one-to-one mapping. If a material uncertainty remains, create a bounded
discovery work unit or resolve it before continuing.

## Mandatory classification and independent review

Every `plan-description.md` must include these sections, even when concise:

```markdown
## UI classification
- UI affected: yes|no
- Rationale: <why>

## Adversarial review
- Artifact: `adversarial-review.md`
- Status: 💤 pending
```

When `UI affected: yes`, the UI validation reference applies and its `Required`
value must be `yes`. Never use `UI affected: no` to avoid browser validation
for an HTML, CSS, template, component, route, or user-facing behavior change.

Before a plan is ready to execute, a **fresh secondary agent** must create
`adversarial-review.md`. Give that agent the request, repository context, and
plan artifacts, but not the planning agent's conclusions. It must identify
every unplanned file, symbol, behavior, test, browser interaction, dependency,
and bug-recovery path needed to execute the request. The plan is rejected
until every finding is resolved and the review verdict is `✅ approved`.

Use this structure:

```markdown
# Adversarial review: <planname>

## Review scope
- Request: <verbatim or precise summary>
- Repository/context inspected: <what was checked>

## Findings

| ID | Missing or over-broad item | Required plan change | Status |
|---|---|---|---|
| AR-01 | No finding recorded yet, or <finding> | N/A, or <specific work unit/goal/story change> | ✅ resolved |

## Verdict
- Status: `✅ approved`
- Rationale: <why no unresolved work remains>
```

Do not allow the planning agent to approve its own review. Re-run the review
after a material scope change or a discovered bug.

When the secondary review approves the plan, synchronize both status fields in
one atomic command (only after the independent reviewer has actually approved
it):

```bash
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" review-status \
  <plan-directory> approved
```

Use `review-status <plan-directory> pending` when reopening the review. The
approved form refuses to proceed while the review still contains an open or
in-progress `AR-` finding. The validator rejects a missing, pending, or
mismatched plan-description status.

Execution order is mandatory: write the complete draft plan first, invoke the
fresh reviewer, wait for its artifact, resolve every finding by revising the
plan, then invoke a fresh reviewer again when revisions were material. Only
after an approved artifact exists may the planning agent run the readiness
validator and create progress trackers.

## 8. Add verification instructions

For every step with verifiable behavior, create a companion file with the same
number and slug:

```text
<goalname>/steps/01-step-<short-slug>-testing.md
```

Omit the file only when there is genuinely nothing to verify, such as a pure
documentation step.

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

## 9. Validate, then create progress trackers

Run the validator before creating trackers or presenting the plan as ready:

```bash
PLANNING_SKILL_DIR="<installed-planning-skill-directory>"
"$PLANNING_SKILL_DIR/scripts/validate-plan.sh" <plan-directory>
```

Do not waive validation failures. Correct the inventory, goal boundary, or
step files and run it again. The validator checks the structural guarantees;
the decomposition review remains required for semantic completeness.

Every implementation, markup, style, configuration, data, or generated work
unit must have a downstream `test` or `verification` unit in the dependency
graph. Add a bounded proof unit rather than asserting that a change is too
small to test.

Create `<planname>/progress.md` for goals and
`<goalname>/progress.md` for steps. Initialize every item as `💤 incomplete`.

The plan-level tracker contains one row per goal:

```markdown
# Progress: <planname>

**Overall progress:** `0%  #### ----------------  100%` 💤

| Goalname | Description | Completion status |
|---|---|---|
| 01-<goal-slug> | <short description> | 💤 incomplete |
```

Each goal-level tracker contains one row per implementation step. Use this
exact four-column format so the helper scripts can update it:

```markdown
# Progress: <goalname>

**Progress:** `0%  #### ----------------  100%` 💤

| Goalname | Stepname | Description | Completion status |
|---|---|---|---|
| 01-<goal-slug> | 01-step-<short-slug> | <short description> | 💤 incomplete |
```

Use these statuses consistently:

- `💤 incomplete` — not started
- `⏳ in progress` — currently being worked on
- `✅ completed` — implementation and all applicable verification passed

Progress percentages count completed items equally. A goal is complete only
when all its steps and applicable verification are complete. The initiative is
complete only when all goals are complete.

## 10. Resume and update the plan

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

Use the bundled scripts on Bash or Zsh instead of rebuilding tracker logic or
patching a plan document:

```bash
PLANNING_SKILL_DIR="<installed-planning-skill-directory>"
"$PLANNING_SKILL_DIR/scripts/create-progress.sh" <goal-directory> <goal-name>
"$PLANNING_SKILL_DIR/scripts/create-plan-progress.sh" <plan-directory>
"$PLANNING_SKILL_DIR/scripts/create-plan.sh" <plan-directory> "<plan title>"
"$PLANNING_SKILL_DIR/scripts/create-work-unit-inventory.sh" <plan-directory>
"$PLANNING_SKILL_DIR/scripts/create-adversarial-review.sh" <plan-directory>
"$PLANNING_SKILL_DIR/scripts/create-ui-validation.sh" <plan-directory> "<browser target or discovery method>"
"$PLANNING_SKILL_DIR/scripts/add-ui-story.sh" <plan-directory> US-01 "<persona>" "<browser actions>" "<direct interaction>" "<expected result>" W01,W02
"$PLANNING_SKILL_DIR/scripts/configure-ui-story-cache.sh" <plan-directory> US-01 "<starting state>" "<direct UI input>" "<target/value>" "<readiness signal>" "<maximum wait>"
"$PLANNING_SKILL_DIR/scripts/add-goal.sh" <plan-directory> 01-<goal> "<title>" "<outcome>"
"$PLANNING_SKILL_DIR/scripts/add-work-unit.sh" <plan-directory> W01 <type> <file|N/A> <scope> <subscope|N/A> "<change>" <dependencies|—> 01-<goal> 01-step-<slug>
"$PLANNING_SKILL_DIR/scripts/add-coverage.sh" <plan-directory> "<outcome or proof>" W01 "<notes>"
"$PLANNING_SKILL_DIR/scripts/update-step.sh" <goal-directory> <step-name> in-progress
"$PLANNING_SKILL_DIR/scripts/update-step.sh" <goal-directory> <step-name> completed
"$PLANNING_SKILL_DIR/scripts/update-progress.sh" <goal-directory>
"$PLANNING_SKILL_DIR/scripts/update-plan-progress.sh" <plan-directory> <goal-name> in-progress
"$PLANNING_SKILL_DIR/scripts/update-plan-progress.sh" <plan-directory> <goal-name> completed
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" title <plan-directory> plan "<title>"
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" section <plan-directory> plan affected-areas -p 6.1: "<first paragraph>" -p 6.2: "<second paragraph>"
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" paragraph <plan-directory> plan -p 6.1: "<replacement>"
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" field <plan-directory> plan 'UI affected' no
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" decomposition-review <plan-directory> completed
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" review-status <plan-directory> approved
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" get <plan-directory> unit:W01 json
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" summary <plan-directory> markdown
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" blast-radius <plan-directory> W01 markdown
```

The creation scripts refuse to overwrite existing trackers. The update
scripts change the requested row and recalculate the relevant progress bar.
`plan-content.sh` supports `markdown`, `text`, and `json` output for summaries
and blast radius, plus `path` for a direct document lookup. Document IDs are
`plan`, `review`, `goal:<goal>`, `step:<goal>/<step>`, and `unit:<WNN>`.
