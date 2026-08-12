# Step: 06-step-independent-grader-regression

## Ownership

- Goal: `20-blinded-seeded-defect-oracle`
- Work unit: `W100`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-review-oracle.sh`
- Primary symbol or file scope: `Independent grader regression`
- Subscope: `N/A`

## Objective

§ 4.1
Verify the standalone grader is covered by blinded protocol tests

## Instructions

§ 5.1
Run the review-oracle regression, including encrypted seeding, target isolation, independent role enforcement, report secrecy, and standalone grader behavior.

## Acceptance criteria

§ 6.1
The blinded protocol test passes and rejects a non-oracle caller.

## Handoff

§ 7.1
Goal 20 has complete source/test coverage for the seeder, launch boundary, grader, and regression suite.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
