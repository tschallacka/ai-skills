# Step: 08-step-atomic-publication

## Ownership

- Goal: `05-protocol-archive`
- Work unit: `W50`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `result publication block`
- Subscope: `N/A`

## Objective

§ 4.1
Stage evaluation and copied artifacts under a private run directory, validate all preconditions, atomically rename on success, and clean up on failure/collision/interruption.

## Instructions

§ 5.1
Use a private staging directory distinct from benchmark/results/{run-id}/{revision}; write evaluation, telemetry, audit, lifecycle, and copied artifacts there; require all mandatory fields and schema-valid telemetry; atomically rename only after checks pass; remove staging on failure or collision.

## Acceptance criteria

§ 6.1
Only `/benchmark/results/<run-id>/<revision>` is visible after a successful rename; all failed preconditions leave no published directory and retain rejection/cleanup evidence under the run temp root.

## Handoff

§ 7.1
Record the changed target, command/output evidence, and unavailable or environment-specific results for the next dependency.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
