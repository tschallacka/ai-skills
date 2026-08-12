# Step: 07-step-control-analysis

## Ownership

- Goal: `07-pilot-release`
- Work unit: `W56`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `iterative-vs-control metric analysis`
- Subscope: `N/A`

## Objective

§ 4.1
Compute token and latency deltas, reviewer event/finding/fix counts, final validation, taint rate, and independent defect detection; fail adoption when evidence is unavailable or the decision rule is not met.

## Instructions

§ 5.1
Write `/tmp/<analysis-run-id>/comparison.json` from the explicitly listed
current-protocol archives in `input-manifest.tsv`. Historical 1.3.1/1.4.1
archives may be listed as immutable context only; they must not be rerun,
retrofitted, or treated as current-protocol controls. Require archive paths,
task hashes, telemetry status, taint causes, `oracle.json`, W57 approval,
thresholds, formulas, and adoption boolean; write `comparison.md` only as a
rendering of `comparison.json`.

## Acceptance criteria

§ 6.1
comparison.json is schema-valid, maps all four archives, contains oracle denominators/rates, telemetry/taint status, threshold outputs, and W57 approval evidence; it exits nonzero when any required input is absent or non-machine-readable.

## Handoff

§ 7.1
Record the changed target, command/output evidence, and unavailable or environment-specific results for the next dependency.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
