# Step: 02-step-alias-expose

## Ownership

- Goal: `04-per-defect-diagnostics`
- Work unit: `W10`
- Type: `config`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `per-defect aliases in metadata/telemetry`
- Subscope: `N/A`

## Objective

§ 4.1
Surface a public projection of per_defect diagnostics into protocol-metadata and telemetry aliases without leaking expected_signal, required_correction, or seed IDs.

## Instructions

§ 5.1
In setup-benchmark.sh, extend the reviewer-state builder (lines 1031-1136) to carry the sanitized per-defect diagnostics into reviewer-state.json, protocol-metadata.json, and telemetry.json, and sequence this after W08's threshold/denominator edits to the same block. Never write seed IDs, expected_signal, or required_correction into any public artifact.

## Acceptance criteria

§ 6.1
protocol-metadata and telemetry expose sanitized per-defect classification and failed-predicate lists with no seed text.

## Handoff

§ 7.1
W11 uses the aliases; goal-05 fixtures assert the sanitized shape.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
