# Step: 02-step-threshold-lifecycle-test

## Ownership

- Goal: `05-regression-fixtures-and-replay`
- Work unit: `W13`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-review-lifecycle.sh`
- Primary symbol or file scope: `threshold reason split case`
- Subscope: `N/A`

## Objective

§ 4.1
Add lifecycle cases asserting MISSING_THRESHOLDS when thresholds are absent and that MISSING_DENOMINATOR does not fire when the oracle reports a valid denominator.

## Instructions

§ 5.1
In test-review-lifecycle.sh, add cases asserting MISSING_THRESHOLDS when thresholds are absent and that MISSING_DENOMINATOR does not fire when the oracle reports a valid denominator.

## Acceptance criteria

§ 6.1
The lifecycle tests enforce the reason split.

## Handoff

§ 7.1
W15 runs the suite.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
