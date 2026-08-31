# Step: 09-step-fixture-contract-test

## Ownership

- Goal: `15-fixture-corpus`
- Work unit: `W99`
- Type: `test`

## Change target

- File: `planning/tests/test-overview-fixtures.sh`
- Primary symbol or file scope: `fixture corpus contract`
- Subscope: `N/A`

## Objective

§ 4.1
Pin that each fixture still carries the edge case it exists for, so a fixture edited for another reason cannot silently stop covering the story that depends on it.

## Instructions

§ 5.1
Write planning/tests/test-overview-fixtures.sh so it asserts, per fixture, the specific edge case that fixture exists for: the orphan, the size exception, the testing-requirement no, the four evidence gaps, the completed shape, the empty approved shape, the three zeroes of the fresh plan, and the two recorded damages. Assert the recorded provenance of the two snapshots as well, so a snapshot replaced without its record fails.

## Acceptance criteria

§ 6.1
The test passes on the corpus as built, and fails with a message naming the fixture when any single edge case is removed from it. It does not merely check the directories exist.

## Handoff

§ 7.1
Goal 15's definition of done is reached: each story's required state exists, is checked in, and is defended by a test that fails when the state is edited away. Goal 09 can record a story pass on a clean checkout.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
