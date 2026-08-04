---
name: planning
description: Use when the user requests a durable plan or a multi-step initiative genuinely needs resumable files, ordered goals and steps, verification instructions, progress trackers, or handoff notes. Do not use for small, self-contained changes or temporary in-chat checklists.
---

# Planning

Use this skill to turn an initiative into a directory of Markdown files that
another agent can resume and execute without reconstructing missing context.

Do not use it for a small, self-contained change or a temporary in-chat plan.

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

## 4. Decompose the initiative into goals

Divide the initiative into a small, ordered set of cohesive goals. Order goals
by dependency when that matters. Do not split a user-visible outcome merely by
technical layer or file type.

Create one directory per goal. The plan must have this structure:

```text
<planname>/
├── plan-description.md
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

Keep ownership clear. Document a shared contract once in
`plan-description.md`; reference it from goals instead of copying it. When a
goal depends on an earlier goal, read that goal's completed handoff before
composing or executing the dependent goal.

## 5. Add working context when needed

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

## 6. Break each goal into steps

Create the smallest ordered steps that together deliver the goal. A step may
change code, write a test, create test data, or perform a verification flow.
One step is valid when the goal is genuinely that small.

Store steps in `<goalname>/steps/` using two-digit, zero-padded prefixes:

```text
01-step-<short-slug>.md
02-step-<short-slug>.md
```

Each step file must contain:

- The owning goal and this step's objective
- Files or areas it touches
- Directly executable implementation instructions
- Acceptance criteria for this step
- Any handoff needed by a later step

If a goal needs many steps, especially dozens or more, review whether it
contains multiple independent outcomes. Split it only when the resulting goals
have distinct ownership, boundaries, and definitions of done. Do not split a
goal solely to reduce its step count.

After decomposing a goal, check that steps fully cover its scope without
overlap. If a material uncertainty remains, resolve it before continuing.

## 7. Add verification instructions

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

## 8. Create progress trackers

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

## 9. Resume and update the plan

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

Use the bundled scripts on Bash or Zsh instead of rebuilding tracker logic:

```bash
PLANNING_SKILL_DIR="<installed-planning-skill-directory>"
"$PLANNING_SKILL_DIR/scripts/create-progress.sh" <goal-directory> <goal-name>
"$PLANNING_SKILL_DIR/scripts/create-plan-progress.sh" <plan-directory>
"$PLANNING_SKILL_DIR/scripts/update-step.sh" <goal-directory> <step-name> in-progress
"$PLANNING_SKILL_DIR/scripts/update-step.sh" <goal-directory> <step-name> completed
"$PLANNING_SKILL_DIR/scripts/update-progress.sh" <goal-directory>
"$PLANNING_SKILL_DIR/scripts/update-plan-progress.sh" <plan-directory> <goal-name> in-progress
"$PLANNING_SKILL_DIR/scripts/update-plan-progress.sh" <plan-directory> <goal-name> completed
```

The creation scripts refuse to overwrite existing trackers. The update
scripts change the requested row and recalculate the relevant progress bar.
