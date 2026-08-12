# Step: 04-step-full-suite

## Ownership

- Goal: `05-regression-fixtures-and-replay`
- Work unit: `W15`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `N/A`
- Subscope: `N/A`

## Objective

§ 4.1
Run test-review-oracle.sh, test-review-lifecycle.sh, test-frozen-replay.sh, the plan validator, and the oracle/lifecycle suites; all must pass.

## Instructions

§ 5.1
Run the full gate: after W02 run the existing single-file/S-location consolidated fixture, after W06 run the counts-dict assertion (aggregate shape unchanged), then run test-review-oracle.sh, test-review-lifecycle.sh, test-frozen-replay.sh, the grader against the frozen archives, and the plan validator. If frozen replay fails, diagnose via the W09 failed-predicate list and correct only unimplemented expectations, never weakening a real predicate to force a pass.

## Acceptance criteria

§ 6.1
All suites pass, the counts dict keeps its asserted shape (plus the new partial key), and the plan validator reports clean.

## Handoff

§ 7.1
This is the final gate; report results against the plan-description definition of done.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
