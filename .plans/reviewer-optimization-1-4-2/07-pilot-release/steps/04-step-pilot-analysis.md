# Step: 04-step-pilot-analysis

## Ownership

- Goal: `07-pilot-release`
- Work unit: `W37`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `pilot comparison and decision rule`
- Subscope: `N/A`

## Objective

§ 4.1
Compare tokens, reviewer events, findings, fixes, final validation, taint rate, independent defect detection, and archive completeness against fresh-review mode; adopt only if the decision rule passes.

## Instructions

§ 5.1
Analyze the pilot and matched control using the fixed formulas: token delta=current−control, latency delta=current−control, percentage delta=delta/control×100, true-positive rate=TP/(TP+FN), independent-catch rate=independent catches/all seeded defects, and taint rate=tainted runs/total runs. Treat missing denominators as unavailable and fail adoption.

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
