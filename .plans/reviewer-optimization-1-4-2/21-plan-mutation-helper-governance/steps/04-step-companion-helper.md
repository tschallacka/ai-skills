# Step: 04-step-companion-helper

## Ownership

- Goal: `21-plan-mutation-helper-governance`
- Work unit: `W93`
- Type: `source`

## Change target

- File: `planning/scripts/create-step-testing.sh`
- Primary symbol or file scope: `Create testing companions through the canonical dispatcher`
- Subscope: `N/A`

## Objective

§ 4.1
Provide atomic helper-only companion creation with strict step validation

## Instructions

§ 5.1
Use the canonical dispatcher to validate the goal directory and step name, create the testing companion atomically, and reject missing or duplicate targets.

## Acceptance criteria

§ 6.1
A valid step receives exactly one companion with the required verification heading; invalid and duplicate requests fail without changing plan files.

## Handoff

§ 7.1
W96 can invoke the companion helper as part of the complete mutation regression suite.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
