# Step: 08-step-telemetry-extraction

## Ownership

- Goal: `06-safeguards-telemetry`
- Work unit: `W53`
- Type: `source`

## Change target

- File: `benchmark/planning/telemetry.sh`
- Primary symbol or file scope: `python extraction block`
- Subscope: `N/A`

## Objective

§ 4.1
Extract exact fields from matched telemetry sources into raw.jsonl, preserving provenance and marking heuristic/unavailable values; do not validate or publish the final telemetry artifact in this step.

## Instructions

§ 5.1
W53 writes only <STAGING_DIR>/telemetry/raw.jsonl plus source/provenance records. It rejects malformed, stale, and ambiguous matches at extraction time with raw rejection evidence, then hands raw records to W60; final schema validation and telemetry.json/telemetry-rejection.json ownership is W60.

## Acceptance criteria

§ 6.1
telemetry.sh writes raw.jsonl only; schema validation writes telemetry.json or telemetry-rejection.json; setup-benchmark.sh consumes only telemetry.json; W50 runs the ordered audit/taint, telemetry, artifact, and rename preconditions.

## Handoff

§ 7.1
Record the changed target, command/output evidence, and unavailable or environment-specific results for the next dependency.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
