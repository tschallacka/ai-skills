# Step: 03-step-plan-mutation-helper-tests

## Ownership

- Goal: `21-plan-mutation-helper-governance`
- Work unit: `W92`
- Type: `test`

## Change target

- File: `planning/tests/test-progress-helpers.sh`
- Primary symbol or file scope: `helper regression fixtures`
- Subscope: `N/A`

## Objective

§ 4.1
Prove the helper-only workflow preserves document and tracker consistency.

## Instructions

§ 5.1
Test helper-only mutations, four-column progress updates, testing-companion creation, progress rebuild, dispatcher validation, duplicate rejection, and a strict multi-command batch.

## Acceptance criteria

§ 6.1
All helper fixtures pass and the structural plan validator accepts the resulting plan.

## Handoff

§ 7.1
Hand helper regression evidence to the final code-to-plan review.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
