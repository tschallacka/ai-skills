# Step: 02-step-installer-tests

## Ownership

- Goal: `07-pilot-release`
- Work unit: `W35`
- Type: `test`

## Change target

- File: `planning/tests/test-installer-manifest.sh`
- Primary symbol or file scope: `v27 manifest test function`
- Subscope: `N/A`

## Objective

§ 4.1
Verify exact manifest coverage, destination mapping, collision/approval failure behavior, and no partial install for the 1.4.2 package.

## Instructions

§ 5.1
Run the manifest/map set-equality test and installer dry-run against a temporary target. Prove benchmark inputs, results, telemetry, capsules, and pilot archives are not installed; prove approval refusal and collision refusal happen before copy.

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
