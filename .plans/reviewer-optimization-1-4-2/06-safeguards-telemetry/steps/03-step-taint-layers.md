# Step: 03-step-taint-layers

## Ownership

- Goal: `06-safeguards-telemetry`
- Work unit: `W30`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `STATUS calculation`
- Subscope: `N/A`

## Objective

§ 4.1
Separate taint causes for access, process audit, telemetry, worker, reviewer, analyzer, and validation failures while preserving raw evidence in evaluation.md.

## Instructions

§ 5.1
Define `taint-causes.json` as an array of independent causes with layer, code, evidence path, timestamp, and actor. Preserve multiple causes; do not apply first-error precedence. Required layers are access, process-audit, telemetry, worker, reviewer, analyzer, and validation.

## Acceptance criteria

§ 6.1
Combined-failure fixtures retain every applicable cause and evidence path in evaluation.md and taint-causes.json; missing telemetry is never reported as worker failure, and analyzer failure is never reported as validation failure.

## Handoff

§ 7.1
Record the changed target, command/output evidence, and unavailable or environment-specific results for the next dependency.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
