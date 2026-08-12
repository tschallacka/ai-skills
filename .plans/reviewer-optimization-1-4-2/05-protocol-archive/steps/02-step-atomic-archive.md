# Step: 02-step-atomic-archive

## Ownership

- Goal: `05-protocol-archive`
- Work unit: `W22`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `evaluation template metadata`
- Subscope: `N/A`

## Objective

§ 4.1
Stage each run under a timestamp/name directory, include skill/version and protocol metadata, and publish atomically only after artifact and telemetry checks pass.

## Instructions

§ 5.1
Work only on `benchmark/planning/setup-benchmark.sh`, targeting `result staging and publication`. Stage each run under a timestamp/name directory, include skill/version and protocol metadata, and publish atomically only after artifact and telemetry checks pass. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
