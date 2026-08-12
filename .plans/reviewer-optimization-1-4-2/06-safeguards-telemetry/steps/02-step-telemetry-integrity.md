# Step: 02-step-telemetry-integrity

## Ownership

- Goal: `06-safeguards-telemetry`
- Work unit: `W29`
- Type: `source`

## Change target

- File: `benchmark/planning/telemetry.sh`
- Primary symbol or file scope: `database discovery`
- Subscope: `N/A`

## Objective

§ 4.1
Resolve configured/current telemetry stores system-independently, match exact worker UUIDs, reject stale or ambiguous matches, and record database path and lookup method.

## Instructions

§ 5.1
Work only on `benchmark/planning/telemetry.sh`, targeting `database discovery and UUID matching`. Resolve configured/current telemetry stores system-independently, match exact worker UUIDs, reject stale or ambiguous matches, and record database path and lookup method. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
