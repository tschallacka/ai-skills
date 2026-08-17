---
name: planning
description: Use when the user requests a durable plan or a multi-step initiative genuinely needs resumable files, ordered goals and steps, verification instructions, progress trackers, or handoff notes. Do not use for small, self-contained changes or temporary in-chat checklists.
---

# Planning

Use this skill to turn an initiative into a directory of Markdown files that
another agent can resume and execute without reconstructing missing context.

Do not use it for a small, self-contained change or a temporary in-chat plan.

## Setup / prerequisites

Every helper command in this skill is written as
`"$PLANNING_SKILL_DIR/scripts/<name>.sh" ...`, where `<installed-planning-skill-directory>`
and `<PLANNING_SKILL_DIR>` both mean **the directory containing this `SKILL.md`**
(the `planning/` directory of the installed skill). Before running any helper,
set it to that path, for example:

```bash
PLANNING_SKILL_DIR="<installed-planning-skill-directory>"   # e.g. the installed planning/ dir
export PLANNING_SKILL_DIR
```

Plans live under a **plans root**, resolved by `scripts/plan-root.sh` (see
section 2): `PLANS_ROOT` if set, else `<project>/.plans`, else `~/.plans`. Set
`PLANS_ROOT` explicitly when automation must never prompt. Angle-bracket
placeholders such as `<plan-directory>`, `<planname>`, `<goal>`, `<step>`,
`<WNN>`, and `<text>` are literal tokens to be substituted with real values,
not literal text.

**Normative language.** Requirement keywords follow RFC 2119 / RFC 8174
meaning: **MUST / MUST NOT** = absolute requirement; **SHOULD / SHOULD NOT** =
recommendation that may be overridden only with a stated reason; **MAY / OPTIONAL**
= permitted. Bare imperatives in the Operating rules are MUSTs. When a rule says
"never", it is a MUST NOT. This convention is stated here once and applies to
this file and the three references.

**Numbered paragraphs.** Plan documents are organized into numbered paragraphs
labeled `§ N.N` (section.number, e.g. `§ 2.1`, `§ 9.2`), maintained by the
helpers. Address one via `-p N.N: <text>`; the helper flags map to sections
(`--description-paragraph … 6.1`, `--goal-paragraph … 5.1`, etc.). The **change
target** of a work unit is its one file plus primary symbol (or file scope),
from its inventory row and step header. The goal **roster** is the goal's
`§9.1` owned-work-units list naming every unit the inventory assigns to it.

When an initiative creates, changes, repairs, or validates any UI component,
page, interaction, visual state, or user-facing flow, read
[`references/ui-user-story-validation.md`](references/ui-user-story-validation.md)
in full before establishing the plan boundary. Its workflow is mandatory for
that plan; it adds browser-driven discovery, a user-story artifact, and a
bug-priority feedback loop.

**Plan reads go through the gated reader, never whole-file `cat`/`Read`.**
Before reading any plan artifact (plan docs, goal/step files, progress,
work-unit inventory, adversarial-review), read
[`references/plan-read-contract.md`](references/plan-read-contract.md) and
follow it. It applies to the main agent and to every subagent: plan content
MUST be read via `plan-context.sh read --plan-dir <PLAN_DIR> --document ID`
(or `--unit WNN`), never by `cat`/`Read`/whole-directory load of a plan file or
the `.plans/` tree. A wholesale plan read is a context-overflow violation; if
the gate cannot serve something, report it as a limitation rather than
bypassing it.

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
- Only invoke this skill when the task explicitly requests a durable plan or
  resumable plan files. The presence of `.plans/`, `brainstorm.md`, or
  plan-shaped files does not by itself authorize loading this skill; a
  subagent must not self-load it just because it recognizes those paths.
- **Subagent skill policy.** When you hand work to a fresh secondary agent
  (adversarial review, reviewer, or any subagent), its starting prompt must
  explicitly say: "Do not load any skill on your own. Use only the skills
  explicitly named in this starting prompt; do not infer a skill from file
  names, directories, or paths (for example `.plans/`, `.brainstorm/`, or
  skill files). If you believe another skill is needed, state it and stop —
  do not load it." Do not spawn a subagent that could read this skill and
  then reload it autonomously.
- **Naming a class does not schedule it.** Mentioning a new class, file, or
  method in a unit's instructions does not create an inventory row for it, and
  that unit's own atomicity check forbids touching another file. If a unit's
  instructions **instruct an edit** to a symbol that is not its own change
  target, an inventory row must own it before any plan depends on it.
  `validate-plan.sh --propagation` checks this (on by default): an instruction
  to edit a well-formed `Class::method` that no inventory row owns is surfaced
  as a WARN (never a blocking FAIL), because the rule cannot tell an edit
  instruction from a seam description in short form. A mere mention of a
  vendor/core seam is the point of naming it and is not flagged.
- **When a helper refuses a call, re-issue the call — never patch the script
  that produced it.** If a guard correctly refuses a malformed invocation,
  fix the invocation and re-run it. Editing the invoking script with `sed` or
  a one-off rewrite to force the call through is how literal shell commands
  end up inside plan prose and paragraphs get truncated. The guard worked; the
  workaround is the defect.
- **Record the reason for a decision as carefully as the decision.** A false
  recorded reason propagates into downstream fixes exactly as a false fact
  does. Verify a claim before writing it into a plan as a fact; when a decision
  is right but the reason is unverified, mark the reason as an assumption and
  verify it.
- **State what a correction replaced and why.** A corrected paragraph should
  say what the earlier version said and why it was wrong (for example: "an
  earlier version of this criterion took that direction from a configured
  parameter; it is not an open question"). This lets the next reviewer verify
  the fix landed instead of re-deriving it, prevents the same question being
  reopened, and makes a stale-wording sweep self-documenting — a `find` hit on
  old wording is instantly classifiable as live text or a deliberate
  corrective reference. The cost is verbosity; it is worth it.

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

**Comment discipline for produced code** (see
[`references/comment-discipline-contract.md`](references/comment-discipline-contract.md)).
Code produced under a plan MUST be self-documenting; comments MUST NOT exceed
three lines, MUST keep only genuinely useful, non-evident programming specifics,
and MUST NOT narrate/duplicate what the code already says. Unneeded comments
MUST be removed. Cross-file discovery MUST use repository-aware lookup (code
graph, symbol search), not comments. The post-implementation-review skill
flags violating comments as review findings.

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

One implementation step owns exactly one work unit. It MUST NOT include a
second source file, second symbol, second test target, or a catch-all such as
"related callers." Make those separate, ordered steps even when the changes
are mechanically small. Do not use globs, directory names, or "all affected
files" as a target.

An exception is allowed only for an inseparable generated-file update. Record
the generator command and every generated file in the step, set its type to
`generated`, and explain why individual review is impossible. Never use this
exception for ordinary source, configuration, test, or documentation edits.

### Goal size limit

A goal owns one coherent, independently demonstrable outcome and contains
**2–10 work units** (MUST). A single-work-unit goal is allowed only for a
genuinely standalone documentation, configuration, discovery, or verification
outcome; state the reason in its `goal.md`. A goal with more than 10 work
units is invalid and MUST be split at the next stable product, contract,
deployability, or ownership boundary. Do not split merely by file type.

Every goal needs its own definition of done that can be demonstrated without
claiming completion of later goals. If it cannot be demonstrated independently,
it is a segment of another goal, not a goal.

### Target reachability gate

Before a work unit may target a template, block, or layout, the plan must
record evidence that the target actually renders on the surface in question. A
file existing is not evidence. Record per target, in order:

1. the file exists, and whether it is core's, a module's, or a theme's;
2. no layout in `app/code` or `vendor` removes the block that renders it
   (`<referenceBlock ... remove="true"/>`);
3. no layout re-points it to another template (`<action method="setTemplate">`
   or a re-registered `<block ... template>`);
4. the theme actually in use for that area resolves to it, including every
   intermediate theme in the inheritance chain;
5. for a module template, whether any theme overrides it.

A goal whose units target templates, blocks, or layouts must own a discovery
unit that records this evidence per target, and that unit's acceptance
criteria must require the recorded evidence — not merely that a search was
performed. Steps 2 and 3 are the ones repeatedly missed: a theme-override
search finds neither.

**Marker pre-check (required first step for any unit whose change target is a
template, and for any template-ambiguous block/layout target).** Static
reachability evidence is necessary but not sufficient: it cannot tell which
route actually renders a target that several themes or a re-pointed layout
could serve. Before building anything on an assumption about which file
renders, add a visible literal marker to the candidate template, confirm which
route and block render it (browser or live block tree), then remove the marker
and record the confirmed surface. This is the only mitigation that reliably
catches a "wrong target surface" defect — the class no validator can detect
because it requires reading live block trees and theme chains.

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

For a normal Unix home directory the default is `~/.plans/<planname>/`. Set
`PLANS_ROOT` to pin a different root (automation always sets it and never
prompts). Keep the planning skill installation and durable plan storage
separate.

Create it with the bundled command; do not create the directory or its initial
documents with a patch. It creates a canonical `plan-description.md` and an
empty work-unit inventory that the other commands can update safely:

```bash
PLANNING_SKILL_DIR="<installed-planning-skill-directory>"
# Bare plan name: the plans root is resolved by plan-root.sh (may prompt the
# first time a plan is created in a project).
"$PLANNING_SKILL_DIR/scripts/create-plan.sh" \
  "<planname>" "<plan title>"

# Explicit path (unchanged behaviour): use the root directly.
"$PLANNING_SKILL_DIR/scripts/create-plan.sh" \
  "$PLANS_ROOT/<planname>" "<plan title>"
```

#### Root resolution (`scripts/plan-root.sh`)

`plan-root.sh resolve` chooses the root in this order:

1. `PLANS_ROOT` if already exported — used verbatim, never prompted.
2. `<project>/.plans` when it is consistent with the skill (its `.env`
   records that `.plans` as the plans root) — default, never prompted.
3. A global directory that already matches a format for this project:
   `~/.plans/<owner>/<repo>` (from the git remote) or
   `~/.plans/<user>/<projectdir>`. Recognition is purely by directory format;
   no marker file is written or read.
4. Otherwise this is the first plan in the project: on an interactive
   terminal the user is asked whether to store globally under `~/.plans` or
   in the project's `./.plans`. When project storage is chosen the user is
   then asked whether to add `/.plans` to the project's `.gitignore`. On a
   non-interactive run the installer defaults to project storage and prints a
   note.

`create-plan.sh <planname>` places the new plan under the resolved root. Use
the flagged `update-plan-content.sh` commands for narrative edits; the helpers
enforce paragraph numbering, spacing, sequencing, and safe content.

`create-plan.sh` git-initializes the plan directory, and every mutating helper
commits the pre-mutation state first — `git -C <planname> log` recovers an
overwritten paragraph. When the plans root is git-excluded from its enclosing
work tree (a project's `/.plans` in `.gitignore`) or sits outside any repo,
`create-plan.sh` initializes one repo at the plans root itself so the whole
plans tree is versioned and cross-plan diffs work; `cleanup-plans.sh` clears
that root history when the last plan is removed, and the next `create-plan.sh`
re-initializes it. Read plan documents only through `plan-content.sh`;
its `find` subcommand locates a literal string across plan documents and
prints every `docid<TAB>§ N.N<TAB>excerpt` match (exits 1 on zero or multiple
hits). Use it before a paragraph-level edit to confirm the target is unique:
`plan-content.sh find <plan-directory> 'Magento_Sales::invoices'`.

**`find` scoping grammar.** The first positional after the pattern is either
`--in <scope>` (one of `plan`, `goals`, `steps`, `units`, `review`, `testing`,
`coverage`, `stories`, `inventory` — an alias for `units`, or `all`) or
`--document <docid>` (one exact document: `plan`, `review`, `goal:<g>`,
`step:<g>/<s>`, `unit:<WNN>`, `coverage`, `stories`, `inventory`, `fixes`,
`fix-keys`, `approval`). `--in` and `--document` are mutually exclusive;
`--document` answers "is this wording at the surface the finding named" and
`--in` sweeps a whole class of documents.

**`find` output formats are for different purposes — pick before you run.**
- `text` (default) — for reading. The match rows are followed by a
  human-readable diagnostic line (`N matches …; narrow the pattern or scope`)
  that is **NOT part of the result set**.
- `json` — for enumeration and any post-processing. It contains only
  `{"matches":[{document,section,excerpt}]}` with **no diagnostic line**. Use
  `json` whenever you intend to feed the output to another command.

**Worked examples:**
```bash
# every document mentioning a phrase, machine-readable:
plan-content.sh find <plan> "<pattern>" --in all --format json | jq -r '.matches[].document'

# did a fix land at THIS surface (not merely somewhere in the plan)?
plan-content.sh find <plan> "<required wording>" --document step:<goal>/<step>
```

> **Do not grep or filter the text output to strip its diagnostic line** — that
> is what `--format json` is for. If you are post-processing, you are in the
> wrong format.

**Exit code is a disambiguation signal, not a pass/fail.** `find` exits `1` on
zero **or** multiple matches — deliberately (a unique target is the goal).
"Exit 1 with matches present" means *narrow the pattern*, not *error*, so a
caller checking only the exit code will misread it. With `json`,
`jq '.matches | length'` gives the count directly.

**A fix is verified by finding the wording at the surface the finding named,
never by finding it somewhere in the plan.** The plan-wide probe
(`--in all`) returns true whenever any other unit happens to mention the same
symbol, which produced false "verified" marks. Scope with `--document <docid>`
(or `--in unit:W24`) to ask the precise question the finding asked.

`find` reaches the `*-testing.md` companions via the `--in testing` scope and
within `--in all` (document id `step:<goal>/<step>-testing`). Always include
companions in a stale-wording sweep: they are where execution actually happens
and were historically the surface most likely to drift. A sweep that relies on
`--in all` without companions is incomplete by construction.

**Reserved characters and identifiers.** Plan narrative MUST NOT contain the
reserved paragraph marker `§` (the helpers reject it) or a Markdown table
separator `|`; input must be LF, not CRLF. Finding IDs must match `^AR-[0-9]+$`
and work-unit IDs `^W[0-9]+$` — `mint-fix-keys.sh` warns per non-conforming
gated row and fails the run if any gated row could not be minted, so a typo
cannot silently disable the fix-key gate; use the exact formats.

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

### 2.2 Reason about the work-unit inventory before choosing goals

`create-plan.sh` already creates an empty `work-unit-inventory.md`. This section
is a **reasoning pass** — enumerate the work units and their ownership *before*
physically creating any goal directories. It forces the agent to reason from
concrete changes upward rather than guessing a few broad goals and writing
generic steps beneath them. It does not create files; the physical inventory
rows and step files are created later by `add-work-unit.sh` **after** the
goal exists (section 2.3).

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
file, so their ownership fields cannot drift. **Create the goal first** (section
2.3), then `add-work-unit.sh` for each of its units — `add-work-unit.sh`
requires the goal to already exist. Do not continue an older plan whose
documents predate the current skill contract. If an update is requested for
such a plan, stop and ask the user to rewrite it with the current helpers
before proceeding; do not retrofit it in place.

Work through this sequence in order. Do not skip a question because the answer
seems obvious:

1. **Expand the requested outcome.** List each user-visible behavior,
   contract, data change, operational concern, and required verification that
   must be true at definition of done.
2. **Discover the change surface.** Inspect the repository, design, and
   environment. List every known file and symbol to create or change. When an
   exact file or symbol is not yet knowable, add a bounded discovery work unit
   first; never place `TBD` into an implementation step. When a target is a
   template, block, or layout, apply the target reachability gate: record how
   the target renders (or why it cannot be confirmed), not just that it exists.
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

Let `create-plan.sh`, `add-coverage.sh`, and `add-work-unit.sh` create and
update the inventory. They enforce the table columns, stable IDs, ownership,
and review checklist. The decomposition review is a checklist of six completed
statements in `work-unit-inventory.md`'s `## Decomposition review` section
(every definition-of-done item maps to work units; every known affected file
and changing symbol has its own work unit; every work unit has exactly one goal
and one step; each goal has 2–10 work units or records an exception; each step
has exactly one work unit and no incidental edits; dependencies form an
acyclic executable order). Mark it complete (only after checking the resulting
rows) with
`update-plan-content.sh --decomposition-review <plan-directory> completed`.

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

  Layout note: the section holds a single summary paragraph `§ 9.1`; the
  per-unit blurbs (`§ 9.2` … `§ 9.N`) come after the Testing-requirement table
  and are managed by `add-work-unit.sh`/`remove-work-unit.sh`. Never author or
  renumber them by hand, and never pass `-p 9.2:`+ to `--goal-section …
  owned-work-units` — that inserts a duplicate ahead of the table.
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

**A criterion that cannot be satisfied is worse than a missing criterion.**
Before writing an acceptance criterion, check that the target can actually
produce the observable it names — a renderer that emits four per-state lines
cannot be asked for a `Backordered` label it has no source for, and a generated
PDF cannot be byte-identical when embedded metadata differs every run. A gate
on an impossible observable fails correct work and passes wrong work. This
class is hard to detect mechanically; once you are looking for it, it is easy
to spot. The `--stale` phrase list flags the loudest wording (`identical`,
`byte-identical`, `pixel-identical`).

**Test-first units need a red baseline.** A unit that authors a test before the
code it tests exists must state its pass condition as an explicit red baseline
(the test fails, then the following source unit makes it pass), not as a green
suite at a position where the code does not exist yet. Record this in the
step's instructions and in the unit's intended change (for example
`type: test-first`), so the executor does not read "test passes" as "done".

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
`verify-fix-keys.sh --claimed-by <session>` warns when the claiming session is
the session recorded as `minted_by`, and a warning is a review finding, not a
pass. If a fixer must mint its own keys to record a finding the reviewer
missed, surface it as an open finding for a fresh review rather than resolving
it on its own authority.

<!-- REVIEWER_SECTION:END mandatory-review -->

### 3.1 Reviewer-gated fix keys

Gated findings bind a reviewer finding to an owning work unit. The
`## Findings` table in `adversarial-review.md` has a
**mandatory-with-blank-allowed** `Work unit` column (the last of five): every
row carries a final `WNN` cell, or an empty/`N/A` cell when the finding needs
no fix key. `update-adversarial-review.sh` mints a
per-(finding, work-unit) HMAC-SHA256 fix key for every gated row and stores only
the derived keys in `fix-keys.json` beside the review file; the secret itself
lives in the private scratch dir `$(planning_tmpdir)/review-fix-keys/<session-id>/`
(`chmod 700` dir, `chmod 600` secret) and never enters the plan. Finding IDs
must match `^AR-[0-9]+$` and work-unit IDs `^W[0-9]+$`: minting warns per
non-conforming gated row and fails the run if any gated row could not be
minted, so a typo cannot silently disable the whole gate. `fix-keys.json`
records `minted_by` (the session that minted; override with `MINTED_BY`), and
`verify-fix-keys.sh --claimed-by <session>` warns when the claiming session is
the minting session (self-certification).

The fixer records which key each fix used in `fixes.md` as tab-separated claim
lines (`finding_id \t work_unit \t key`, one per gated pair). The approval gate
runs `verify-fix-keys.sh` before `--review-status approved` flips the verdict:
every gated pair must be claimed with a matching key, then the session secret
dir is removed (invalidation) so a stale `fix-keys.json` fails closed on
re-approval. Plans without `fix-keys.json` (ungated) and plans whose findings
all carry no work unit approve without verification.

#### Key reuse and rotation

Fix keys are scoped to one review session. Reuse the same derived key within a
session for repeated claims on the same (finding, work-unit) pair: minting is
idempotent while the session dir exists, so re-running `update-adversarial-review.sh`
re-derives identical keys. A new secret is minted only when the session dir is
gone — which happens at approval (invalidation by the approval gate); keys from
an invalidated session fail verification (stale keys never pass), and
re-approval of such a plan refuses. Rotating the secret within a live session
is never done.

### 3.2 Add verification instructions

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

### 3.3 Validate, then create progress trackers

Run the validator before creating trackers or presenting the plan as ready:

```bash
PLANNING_SKILL_DIR="<installed-planning-skill-directory>"
"$PLANNING_SKILL_DIR/scripts/validate-plan.sh" <plan-directory>
"$PLANNING_SKILL_DIR/scripts/validate-plan.sh" --propagation <plan-directory>   # surface-consistency checks (on by default); --no-propagation disables it
"$PLANNING_SKILL_DIR/scripts/validate-plan.sh" --stale <file-of-phrases> <plan-directory>   # fail when a listed phrase appears in an unmarked paragraph
"$PLANNING_SKILL_DIR/scripts/validate-plan.sh" --stale default <plan-directory>              # bundled case-count phrase list; sweeps companions too
```

`--propagation` encodes the surface rule (§ "Resolving a finding") and runs
by default. It flags a verification unit that grades a sibling with no
dependency path to it (honouring deliberate reverse/baseline orderings and
transitive ordering), a graph leaf in a goal that owns a verification unit, and
a goal whose §9.x roster does not match the units the inventory assigns to it.
It also WARNs (never blocks) when a unit's instructions mention a project
symbol (one whose namespace root or path prefix the plan edits) that no
inventory row owns — this rule cannot distinguish "edit this" from "this is
where we attach" from text alone, so it is a skimmable signal, not a gate. It
does not flag mere vendor/core seams (`Magento\...`, `Amasty\...`,
`Vendor_Module::path` templates), `X::class` constants, or cross-plan
references. `--no-propagation` disables it. `--stale` turns the "sweep for the
old wording" discipline into a gate: a phrase listed in the file fails unless
every paragraph containing it also records a history marker such as
"previously" or "an earlier version". `--stale default` runs a bundled
case-count phrase list (`all four`, `the six states`, etc.) — case-count
wording is the anti-pattern because it drifts when a case is added, and "every
case enumerated in the instructions" is the drift-proof form. The stale sweep
covers the same documents as `find --in all`, including the `*-testing.md`
companions. Add a closed finding's old wording to the phrase file so each fix
becomes a permanent regression guard rather than a one-time correction.

**Coverage rows.** The Definition-of-done coverage table's header is
`Required outcome or proof | Work unit IDs` — it deliberately maps both
outcomes *and* their proofs to units, so crediting a `test` or `verification`
unit is the sanctioned convention, not drift. A coverage row should name the
unit that produces the outcome **as well as** the one that proves it; this is
enforced by review, not by the validator (there is no mechanical form that
avoids false warnings). A testing companion may reference a same-goal
`test`/`verification` unit ("automated tests: covered by WNN") — that is
proof-coverage prose, not a dependency claim.

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
"$PLANNING_SKILL_DIR/scripts/plan-root.sh" resolve   # prints the resolved plans root; prompts on first use in a project
"$PLANNING_SKILL_DIR/scripts/create-adversarial-review.sh" <plan-directory>
"$PLANNING_SKILL_DIR/scripts/create-ui-validation.sh" <plan-directory> "<browser target or discovery method>"
"$PLANNING_SKILL_DIR/scripts/add-ui-story.sh" <plan-directory> US-01 "<persona>" "<browser actions>" "<direct interaction>" "<expected result>" W01,W02
"$PLANNING_SKILL_DIR/scripts/configure-ui-story-cache.sh" <plan-directory> US-01 "<starting state>" "<direct UI input>" "<target/value>" "<readiness signal>" "<maximum wait>"
"$PLANNING_SKILL_DIR/scripts/add-goal.sh" <plan-directory> 01-<goal> "<title>" "<outcome>"
"$PLANNING_SKILL_DIR/scripts/add-work-unit.sh" <plan-directory> W01 <type> <file|N/A> <scope> <subscope|N/A> "<change>" <dependencies|—> 01-<goal> 01-step-<slug>
"$PLANNING_SKILL_DIR/scripts/update-work-unit.sh" <plan-directory> W01 --depends-on "W23,W24"   # change scope/file/type/depends-on/description in place; retargeting lists the verification units that grade it
Ordering note: goals and steps only append (`NN-kebab-case` is enforced, no
renumbering helper exists). Appending plus a prose "execution order differs
from step numbering" note is the sanctioned pattern. Reviewers must not reject
ordering prose that accompanies recorded dependency edges.
"$PLANNING_SKILL_DIR/scripts/add-coverage.sh" <plan-directory> "<outcome or proof>" W01 "<notes>"            # append a coverage row
"$PLANNING_SKILL_DIR/scripts/add-coverage.sh" <plan-directory> "<outcome or proof>" W01,W02 "<notes>" --replace  # amend (collapses duplicate rows for the same outcome)
"$PLANNING_SKILL_DIR/scripts/verify-target.sh" <plan-directory> W01 [--repo <root>]   # static reachability check: file exists, layout removes/re-points the block, theme override
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
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --delete-paragraph <plan-directory> step:01-g/01-step-a 5.2   # delete one paragraph; later labels in the section renumber
"$PLANNING_SKILL_DIR/scripts/update-work-unit.sh" <plan-directory> W88 --scope "RequestEmployeeSet::forCustomer()"   # --scope is the flag form of the scope positional
"$PLANNING_SKILL_DIR/scripts/update-work-unit.sh" <plan-directory> W88 --type source --description "<new intended change>"   # amend type/description in place
"$PLANNING_SKILL_DIR/scripts/create-adversarial-review.sh" <plan-directory>
"$PLANNING_SKILL_DIR/scripts/update-adversarial-review.sh" <plan-directory> --file review.csv      # rewrite the Findings table from a CSV file
"$PLANNING_SKILL_DIR/scripts/update-adversarial-review.sh" <plan-directory> --cycle 7              # archive the prior Findings table into adversarial-review-history.md under Cycle 7
"$PLANNING_SKILL_DIR/scripts/mint-fix-keys.sh" <plan-directory>                                     # (re)derive per-(finding,work-unit) fix keys into fix-keys.json
"$PLANNING_SKILL_DIR/scripts/verify-fix-keys.sh" <plan-directory> [--claimed-by <session>]          # verify fixes.md claims against fix-keys.json; warn on self-certification
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --field <plan-directory> plan 'UI affected' no
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --decomposition-review <plan-directory> completed
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --review-status <plan-directory> approved
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" get <plan-directory> unit:W01 json
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" summary <plan-directory> markdown
"$PLANNING_SKILL_DIR/scripts/plan-content.sh" blast-radius <plan-directory> W01 markdown
"$PLANNING_SKILL_DIR/scripts/create-step-testing.sh" <goal-directory> <step-name> "<instructions>"
"$PLANNING_SKILL_DIR/scripts/create-step-testing.sh" <goal-directory> <step-name> "<instructions>" --overwrite   # replace a companion; input is validated before any file is touched
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

### 4.6 v27 replacement package handoff

The v27 replacement package is repository-owned until its closure plan is
approved. Its finite installable boundary is the six-column
`planning/V27-PACKAGE-MANIFEST.txt`; `planning/V27-PACKAGE-MAP.tsv` is the
source/destination ownership record and the two repository-root v27 brainstorm
inputs are source-only. The package contains the v27 contract, benchmark and
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
target command and the exact manifest: `bash install.sh --install-skill
planning --target TARGET --approval yes`. Before that boundary, use
`--print-skill-files planning --format=tsv` and
`--resolve-source planning RELATIVE_PATH` only for inspection. Declined
approval and destination collisions fail before copy or backup; preserve the
target, record the failure, resolve the collision or approval decision, and
resume the same manifest rather than installing a partial package.
