# Goal 03 / Step 02: Test approval-state regression

## Ownership

- Goal: `03-regression-and-release-gates`
- Work unit: `W08`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-review-lifecycle.sh`
- Primary symbol or file scope: `approval state contract test`
- Subscope: `approval-conflict fixtures`

## Objective

Prevent completed-but-unapproved reviews from being labeled adoptable.

## Instructions

Test true, false, missing, duplicate, and conflicting approval artifacts and
assert the explicit state truth table.

## Acceptance criteria

False approval remains gradeable but is never adoptable; conflicts fail closed.

## Handoff

W11 uses these assertions in the full current-protocol gate.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
