# Step: 04-step-prove-equivalence

## Ownership

- Goal: `01-plan-dir-synonym`
- Work unit: `W04`
- Type: `test`

## Change target

- File: `planning/tests/test-flag-form-equivalence.sh`
- Primary symbol or file scope: `differential proof`
- Subscope: `N/A`

## Objective

§ 4.1
One case per converted helper: positional and --plan-dir produce identical trees, output and exit status.

## Instructions

§ 5.1
Add one differential case per converted helper, each asserting the invocation had an effect as well as matching output.

## Acceptance criteria

§ 6.1
test-plan-dir-synonym.sh passes, and revoking a hoist or hoisting to the wrong slot fails it.

## Handoff

§ 7.1
Goal 02 can proceed against a suite that proves this surface.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
