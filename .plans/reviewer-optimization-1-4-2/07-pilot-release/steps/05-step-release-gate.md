# Step: 05-step-release-gate

## Ownership

- Goal: `07-pilot-release`
- Work unit: `W38`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `full package and plan readiness validation`
- Subscope: `N/A`

## Objective

§ 4.1
Run the complete helper, manifest, contract-test, and protocol validation suite before release.

## Instructions

§ 5.1
Run validators and require W50/W51 publication evidence, W52/W53/W54/W60 telemetry evidence, W57 final-independent-gate.json, W58 oracle.json, W59 comparison.json, and a fresh approved adversarial-review.md. Reject if any artifact is absent, schema-invalid, tainted, or not attributable to the selected four archives.

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
