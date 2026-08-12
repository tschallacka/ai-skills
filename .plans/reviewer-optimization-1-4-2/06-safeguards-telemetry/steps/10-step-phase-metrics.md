# Step: 10-step-phase-metrics

## Ownership

- Goal: `06-safeguards-telemetry`
- Work unit: `W60`
- Type: `source`

## Change target

- File: `benchmark/planning/telemetry.sh`
- Primary symbol or file scope: `phase metrics extraction block`
- Subscope: `N/A`

## Objective

§ 4.1
Augment raw telemetry with phase metrics, perform the single final schema validation, and publish telemetry.json or telemetry-rejection.json for archive status synthesis.

## Instructions

§ 5.1
W60 consumes W53 raw.jsonl, adds phase/reviewer/lifecycle metrics, validates the complete object against W52 telemetry-schema.json, writes <STAGING_DIR>/telemetry/telemetry.json on success or <STAGING_DIR>/telemetry/telemetry-rejection.json on failure, and returns nonzero on rejection. setup-benchmark.sh and W50 consume only this final result.

## Acceptance criteria

§ 6.1
W53 is the sole raw extractor; W60 is the sole augmentation/final-validator; no later writer modifies telemetry.json. A missing raw record, schema mismatch, or metric failure produces telemetry-rejection.json and blocks publication.

## Handoff

§ 7.1
Hand off schema-validated phase metrics, provenance, exact/heuristic/unavailable classifications, and archive path to W32, W50, and W38.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
