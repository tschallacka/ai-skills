<!-- MODE: PROD -->
> Generated from `skill-source.txt` by `scripts/generate-skill-docs.sh` — do not edit.
> Part 2 of 4 — create the plan directory.
>
> Before treating this part as read: find the line below matching
> `<!-- SKILL-LOAD-PROOF part=part-2 token=... -->` — its position moves on every
> regeneration — and run `planning/scripts/verify-skill-load.sh --part part-2
> --token <the-token-you-found>` before continuing. Naming a token is not
> enough; the command must succeed. If it refuses, you have not finished
> reading this part.

## 2. Create the plan directory

Choose a short, descriptive, kebab-case `<planname>` such as
`checkout-totals-own-page`.

Create the plan under the user-owned `.plans/` root resolved by the helper:

```text
<plans-root>/<planname>/
```

For a normal Unix home directory the global default is `${XDG_CONFIG_HOME:-~/.config}/tsch-ai-skills/plans/<planname>/`. Set
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
   `${XDG_CONFIG_HOME:-~/.config}/tsch-ai-skills/plans/<owner>/<repo>` (from the git remote) or
   `${XDG_CONFIG_HOME:-~/.config}/tsch-ai-skills/plans/<user>/<projectdir>`. Recognition is purely by directory format;
   no marker file is written or read.
4. Otherwise this is the first plan in the project: on an interactive
   terminal the user is asked whether to store globally under the tsch-ai-skills XDG home or
   in the project's `./.plans`. When project storage is chosen the user is
   then asked whether to add `/.plans` to the project's `.gitignore`. On a
   non-interactive run the installer defaults to project storage and prints a
   note.

`create-plan.sh <planname>` places the new plan under the resolved root. Use
the flagged `update-plan-content.sh` commands for narrative edits; the helpers
enforce paragraph numbering, spacing, sequencing, and safe content.

`create-plan.sh` git-initializes a repository for the plan and records which one
in the plan's `.env` as `PLAN_SNAPSHOT_REPO`. Every mutating helper commits the
pre-mutation state into that repository first, so `git -C <the recorded repo>
log` recovers an overwritten paragraph. When the plans root is git-excluded from
its enclosing work tree (a project's `/.plans` in `.gitignore`) or sits outside
any repo, that repository is the plans root itself, so the whole plans tree is
versioned and cross-plan diffs work; `cleanup-plans.sh` clears that root history
when the last plan is removed, and the next `create-plan.sh` re-initializes it.
A plan tracked inside a repository you own is the one layout with no
per-mutation snapshots: `PLAN_SNAPSHOT_REPO` is empty there, because a commit
per helper call does not belong in your project's history. Gitignore the plans
root to get the undo back. Read plan documents only through `plan-content.sh`;
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
plan-content.sh find <plan> "<pattern>" --in all --format json | rjq -r '.matches[].document'

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
`rjq '.matches | length'` gives the count directly.

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
- **Environment facts:** the host or URL to verify on, the auth route if the
  application requires one, and the order in which steps verify against the
  running application. This is what lets verification steps name a reachable
  endpoint instead of leaving them dangling. Seeded by `create-plan.sh` as
  § 9.1; the validator requires the section, and the P0 serve check (see
  § 3.3) relies on it.
- **Approach decisions:** mechanism choices as prose — where each change
  lives and why, and alternatives considered and rejected. Seeded by
  `create-plan.sh` as § 10.1; the validator requires the section.

Use one clear section per topic. Do not duplicate goal-specific implementation
details here; put them in the owning goal.

The canonical plan description has replaceable `title`, `current-state`,
`desired-outcome`, `approach`, `approach-decisions`, `scope`, `affected-areas`,
`constraints-and-decisions`, `risks-and-open-questions`, and
`environment-facts` sections. Replace
their narrative content through flagged `update-plan-content.sh` targets; use
`--title` for the document title and `--field` for structured values such as
`UI affected`.

**A goal document's ids are not the plan description's, and neither set is
derivable from the headings.** `-gs`/`--goal-section` accepts
`current-state-and-prior-goal-handoffs`, `outcome-and-definition-of-done`,
`why-this-goal-is-needed`, `scope`, `affected-areas`,
`dependencies-and-handoffs`, `implementation-approach-risks-and-edge-cases`,
`owned-work-units`, and `goal-size-exception`. Only `scope` and
`affected-areas` are spelled the same in both documents; a goal's approach
section is `implementation-approach-risks-and-edge-cases`, not `approach`.
Guessing wrong is safe — the refusal lists every valid id for that document
kind — but the lists are here so a first attempt need not fail to find them.

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
"$PLANNING_SKILL_DIR/scripts/add-work-unit.sh" <plan-directory> \
  --id W01 --type source --file path/to/file --scope 'Class::method()' \
  --subscope N/A --change "<one concrete change>" --depends-on '—' \
  --goal 01-<goal> --step 01-step-<slug>
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
   split candidates at the first stable boundary. When a late approach change
   pushes a goal past the cap, prefer splitting the goal at a stable outcome
   boundary over widening a work unit.
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

<!-- SKILL-LOAD-PROOF part=part-2 token=f044fc4b9a109e6c -->

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
