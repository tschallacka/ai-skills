# Step: 02-step-split-denominator-reasons

## Ownership

- Goal: `03-state-threshold-fix`
- Work unit: `W08`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `reviewer-state threshold/denominator reasons`
- Subscope: `N/A`

## Objective

§ 4.1
Split MISSING_DENOMINATOR from MISSING_THRESHOLDS: MISSING_DENOMINATOR fires only when the oracle denominator is absent/invalid/zero; MISSING_THRESHOLDS fires only when threshold values are absent; record real thresholds in state and metadata.

## Instructions

§ 5.1
In setup-benchmark.sh reviewer-state synthesis (lines 1031-1136), split the guard: add MISSING_DENOMINATOR only when the oracle denominator is absent, invalid, or <=0; add MISSING_THRESHOLDS only when threshold values are None.

## Acceptance criteria

§ 6.1
A run with denominator 3 and configured thresholds records neither reason; a run with missing thresholds records MISSING_THRESHOLDS only.

## Handoff

§ 7.1
Re-synthesize fresh/iterative state to confirm MISSING_DENOMINATOR disappears; W13 pins this.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
