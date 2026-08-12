# Step: 04-step-benchmark-contract

## Ownership

- Goal: `02-review-lifecycle`
- Work unit: `W09`
- Type: `docs`

## Change target

- File: `benchmark/planning/benchmark-test.md`
- Primary symbol or file scope: `review protocol requirements`
- Subscope: `N/A`

## Objective

§ 4.1
Document iterative mode inputs, hard limits, default-mode compatibility, and acceptance criteria for Reviewer B independent defect detection.

## Instructions

§ 5.1
Work only on `benchmark/planning/benchmark-test.md`, targeting `review protocol requirements`. Document iterative mode inputs, hard limits, default-mode compatibility, and acceptance criteria for Reviewer B independent defect detection. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

## Acceptance criteria

§ 6.1
The named target has the required behavior, its output is bounded and reproducible, and the companion or downstream verification can observe the result without an unnamed change.

## Handoff

§ 7.1
Record the changed target, command/output evidence, and unavailable or environment-specific results for the next dependency.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
