# Step: 01-step-export-thresholds

## Ownership

- Goal: `03-state-threshold-fix`
- Work unit: `W07`
- Type: `source`

## Change target

- File: `benchmark/planning/run-benchmark.sh`
- Primary symbol or file scope: `threshold export before setup`
- Subscope: `N/A`

## Objective

§ 4.1
Export SEMANTIC_THRESHOLD and INDEPENDENT_THRESHOLD (from config or passed values) before invoking setup-benchmark.sh so the state synthesizer receives real values instead of empty strings.

## Instructions

§ 5.1
In run-benchmark.sh, before invoking setup-benchmark.sh, export SEMANTIC_THRESHOLD=1.0 and INDEPENDENT_THRESHOLD=1.0 so the state synthesizer receives real non-empty values; document that these defaults match the lifecycle-test values.

## Acceptance criteria

§ 6.1
A seeded cohort run records non-null semantic_threshold and independent_threshold in protocol-metadata and telemetry.

## Handoff

§ 7.1
W08 reads the exported thresholds; W13 tests the resulting reasons.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
