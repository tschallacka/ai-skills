# Goal: Enforce helper-only plan mutations

## Current state and prior-goal handoffs

§ 2.1
Plan helpers exist for goals, work units, content, review status, and progress, but there is no canonical dispatcher or helper for testing companions and adding progress rows.

## Outcome and definition of done

§ 3.1
All durable plan mutations use the helper dispatcher or its named helpers; testing companions and progress rows are created through helpers; helper regression tests and structural validation pass.

## Why this goal is needed

§ 4.1
Direct edits caused tracker inconsistency and bypassed the plan mutation contract. A canonical helper path makes durable plan state reproducible and reviewable.

## Scope

§ 5.1
Include helper dispatch, testing-companion creation, progress-row creation/rebuild, status/review dispatch, strict bulk mutation guidance, protocol documentation, and regression tests. Exclude source-code edits outside planning helpers.

## Affected files, systems, data, and interfaces

§ 6.1
W90 updates planning/SKILL.md; W91 updates planning/scripts/plan-mutate.sh; W92 updates planning/tests/test-progress-helpers.sh; W93 updates planning/scripts/create-step-testing.sh; W94 updates planning/scripts/rebuild-plan-progress.sh; W95 updates planning/scripts/add-adversarial-finding.sh; W96 verifies the complete helper workflow and package records.

## Dependencies and handoffs

§ 7.1
Depends on W84 and hands a helper-only mutation contract to every future plan worker and monitor.

## Implementation approach, risks, and edge cases

§ 8.1
Use atomic helper writes, reject duplicate rows and unsafe values, expose one dispatcher, and run validation after durable mutations. For many updates, generate one strict temporary executable batch of helper calls; a failed command stops the batch and leaves completion status unchanged.

## Owned work units

§ 9.1
W90 — Document the helper-only plan mutation protocol. W91 — Implement the canonical mutation dispatcher. W92 — Test helper-only mutations and tracker consistency. W93 — Provide atomic testing-companion creation. W94 — Provide atomic plan-progress rebuilding. W95 — Provide validated adversarial-finding insertion. W96 — Run complete helper regression coverage. W97 — Bootstrap plan progress when adding the first goal. W99 — Verify the plan-helper lifecycle regression.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The helper workflow changes durable plan files and requires regression coverage. |

§ 9.2
`W91` — Provide the canonical helper dispatcher for goal, work-unit, testing-companion, progress, status, review, and validation mutations.

§ 9.3
`W92` — Test helper-only mutations, four-column progress updates, testing-companion creation, and validation dispatch.

§ 9.4
`W93` — Provide atomic helper-only companion creation with strict step validation

§ 9.5
`W94` — Provide atomic aggregate progress rebuild after durable mutations

§ 9.6
`W95` — Provide atomic review-finding insertion with explicit status handling

§ 9.7
`W96` — Cover companion, progress, rebuild, finding, and dispatcher behavior

§ 9.8
`W97` — Create missing plan progress through the canonical helper before aggregate rebuild

§ 9.9
`W99` — Verify add-goal bootstraps progress and all plan mutations remain valid

§ 9.10


§ 9.11


## Goal-size exception

§ 11.1
Not applicable: protocol, helper implementation, and regression testing have separate ownership.
