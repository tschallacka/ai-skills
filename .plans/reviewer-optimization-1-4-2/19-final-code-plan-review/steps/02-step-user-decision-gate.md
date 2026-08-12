# Step: 02-step-user-decision-gate

## Ownership

- Goal: `19-final-code-plan-review`
- Work unit: `W85`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `user disposition record`
- Subscope: `N/A`

## Objective

Ask the user whether each material final-review gap should amend the plan or
remain as an explicit accepted exception.

## Instructions

Present evidence, impact, and options to the user. If amendment is requested,
create complete linked plan state with dependencies, tests, progress, and
review evidence, then rerun validators. If an exception is accepted, record
the exact scope and rationale. Do not infer the user's decision.

## Acceptance criteria

Every material discrepancy has an explicit user disposition, and any requested
amendment passes structural and adversarial validation before completion.

## Handoff

§ 7.1
The user instructed continuation of the remaining goals; this is recorded as disposition to implement and validate the documented oracle blocker. Goal 20 now supplies the blinded seeder, target isolation checks, independent grader, retained report, package entries, and passing tests. The refreshed validator passes with 96 work units across 21 goals.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
