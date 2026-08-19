# Step: 01-step-confirm-hoister

## Ownership

- Goal: `01-plan-dir-synonym`
- Work unit: `W01`
- Type: `source`

## Change target

- File: `planning/scripts/plan-document-lib.sh`
- Primary symbol or file scope: `plan_hoist_plan_dir`
- Subscope: `N/A`

## Objective

§ 4.1
No change; the hoister already exists and is the seam the other units use.

## Instructions

§ 5.1
Exercise plan_hoist_plan_dir on four shapes: the flag at position 1, at position 2, the = form, and absent.

## Acceptance criteria

§ 6.1
Each shape prints the argument list with the value in the requested slot, including a path containing a space.

## Handoff

§ 7.1
The other units may treat the hoister as a working seam.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
