# Step: 09-step-plan-helper-regression

## Ownership

- Goal: `21-plan-mutation-helper-governance`
- Work unit: `W99`
- Type: `test`

## Change target

- File: `planning/tests/test-plan-commands.sh`
- Primary symbol or file scope: `Plan helper lifecycle regression`
- Subscope: `N/A`

## Objective

§ 4.1
Verify add-goal bootstraps progress and all plan mutations remain valid

## Instructions

§ 5.1
Run the plan-command regression and confirm first-goal creation, content mutation, review approval, validation, and missing-companion rejection all remain enforced.

## Acceptance criteria

§ 6.1
The complete planning command regression passes after the add-goal bootstrap change.

## Handoff

§ 7.1
Goal 21 remains structurally valid with all helper files represented by work units.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
