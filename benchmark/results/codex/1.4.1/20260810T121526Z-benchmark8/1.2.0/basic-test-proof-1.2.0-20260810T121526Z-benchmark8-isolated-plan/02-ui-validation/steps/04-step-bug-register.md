# Step 04: bug register

## Ownership
- Goal: `02-ui-validation`
- Work unit: `W13`
- Type: `docs`

## Change target
- File: `bugs.md`
- Primary symbol or file scope: `UI bug register`
- Subscope: `N/A`

## Objective
Maintain bug-feedback-loop traceability for US-01.

## Instructions
1. Keep the prescribed bug table header with no data row when no browser bug was observed; if W05 finds a bug, add its evidence and linked investigation, fix, and retest goals without deleting history.

## Acceptance criteria
- The register is empty-but-structured for this proof and cannot imply an unobserved pass or bug.

## Handoff
- W08 records the no-bug-observed state and the future escalation rule.

## Atomicity check
- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
