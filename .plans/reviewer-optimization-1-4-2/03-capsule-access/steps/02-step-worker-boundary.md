# Step: 02-step-worker-boundary

## Ownership

- Goal: `03-capsule-access`
- Work unit: `W12`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `generated start-worker.sh access boundary`
- Subscope: `N/A`

## Objective

§ 4.1
Launch workers with capsule-only filesystem scope, create per-worker .bm-vars and wrapper paths, and record command/path access attempts for tainting.

## Instructions

§ 5.1
Work only on `benchmark/planning/setup-benchmark.sh`, targeting `generated start-worker.sh access boundary`. Launch workers with capsule-only filesystem scope, create per-worker .bm-vars and wrapper paths, and record command/path access attempts for tainting. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
