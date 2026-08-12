# Step: 06-step-safeguard-tests

## Ownership

- Goal: `06-safeguards-telemetry`
- Work unit: `W33`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-safeguards.sh`
- Primary symbol or file scope: `process test function`
- Subscope: `N/A`

## Objective

§ 4.1
Prove no worker/reviewer/child survives interruption, ambiguous telemetry is rejected, taint causes remain separate, and archives are not published partially.

## Instructions

§ 5.1
Work only on `benchmark/planning/tests/test-safeguards.sh`, targeting `process test function`. Prove no worker/reviewer/child survives interruption and combined failures retain separate taint causes. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
