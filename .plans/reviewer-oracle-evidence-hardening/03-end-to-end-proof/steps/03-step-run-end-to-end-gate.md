# Step: 03-step-run-end-to-end-gate

## Ownership

- Goal: `03-end-to-end-proof`
- Work unit: `W10`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `bounded seeded benchmark and completion gate`
- Subscope: `N/A`

## Objective

§ 4.1
Run focused contracts plus one iterative and one fresh current-protocol control, then verify 3/3 consolidated semantic and independent catches or fail closed with an explicit unavailable reason.

## Instructions

§ 5.1
Run the focused suite under the resource cap, then execute exactly one iterative and one fresh current-protocol control with fixed task, seed, thresholds, isolated roots, and preserved archives. Inspect evaluation, oracle, reviewer-state, lifecycle, provenance, and comparison artifacts.

## Acceptance criteria

§ 6.1
Pass requires complete attributable archives and 3/3 semantic and independent catches. Any unavailable, malformed, tainted, or below-threshold result is recorded as fail-closed; no adoption claim is made.

## Handoff

§ 7.1
Final handoff includes command output, run IDs, archive paths, state reasons, validator output, and any bounded-control timeout evidence.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
