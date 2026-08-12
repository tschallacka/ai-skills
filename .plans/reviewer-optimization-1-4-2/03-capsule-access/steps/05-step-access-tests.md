# Step: 05-step-access-tests

## Ownership

- Goal: `03-capsule-access`
- Work unit: `W15`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-capsule-access.sh`
- Primary symbol or file scope: `capsule access test functions`
- Subscope: `N/A`

## Objective

§ 4.1
Verify allowed files are readable, unallowlisted source and prior result roots are unavailable, and escape attempts create tainted evidence.

## Instructions

§ 5.1
Work only on `benchmark/planning/tests/test-capsule-access.sh`, targeting `capsule access test functions`. Verify allowed files are readable, unallowlisted source and prior result roots are unavailable, and escape attempts create tainted evidence. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
