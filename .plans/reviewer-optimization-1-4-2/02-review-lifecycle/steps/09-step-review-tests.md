# Step: 09-step-review-tests

## Ownership

- Goal: `02-review-lifecycle`
- Work unit: `W42`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-review-lifecycle.sh`
- Primary symbol or file scope: `review lifecycle test functions`
- Subscope: `N/A`

## Objective

§ 4.1
Test option validation, Reviewer A ownership limits, fresh Reviewer B isolation, handoff artifacts, final approval prohibition, and interruption propagation.

## Instructions

§ 5.1
Work only on `benchmark/planning/tests/test-review-lifecycle.sh`, targeting `review lifecycle test functions`. Test option validation, Reviewer A ownership limits, fresh Reviewer B isolation, handoff artifacts, final approval prohibition, and interruption propagation. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
