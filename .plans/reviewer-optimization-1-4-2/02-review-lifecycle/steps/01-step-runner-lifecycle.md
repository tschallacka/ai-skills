# Step: 01-step-runner-lifecycle

## Ownership

- Goal: `02-review-lifecycle`
- Work unit: `W06`
- Type: `source`

## Change target

- File: `benchmark/planning/run-benchmark.sh`
- Primary symbol or file scope: `review lifecycle state declarations`
- Subscope: `N/A`

## Objective

§ 4.1
Define bounded review-cycle state, reviewer session records, pass/cycle limits, and lifecycle event fields; process launch, handoff, and signal cleanup are owned by separate steps.

## Instructions

§ 5.1
Work only on `benchmark/planning/run-benchmark.sh`, targeting `review lifecycle state declarations`. Define bounded review-cycle state, reviewer session records, pass/cycle limits, and lifecycle event fields. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
