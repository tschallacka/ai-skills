# Step: 05-step-runner-tests

## Ownership

- Goal: `02-review-lifecycle`
- Work unit: `W10`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-review-runner.sh`
- Primary symbol or file scope: `review-cycle option test`
- Subscope: `N/A`

## Objective

§ 4.1
Exercise option validation, fresh-review fallback, maximum-pass termination, and lifecycle metadata in a bounded harness fixture.

## Instructions

§ 5.1
Work only on `benchmark/planning/run-benchmark.sh`, targeting `review-cycle option and lifecycle output`. Exercise option validation, fresh-review fallback, maximum-pass termination, and lifecycle metadata in a bounded harness fixture. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
