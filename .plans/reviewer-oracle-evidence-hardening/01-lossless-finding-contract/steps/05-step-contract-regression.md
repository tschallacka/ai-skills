# Step: 05-step-contract-regression

## Ownership

- Goal: `01-lossless-finding-contract`
- Work unit: `W11`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-review-oracle.sh`
- Primary symbol or file scope: `complete finding envelope fixture`
- Subscope: `N/A`

## Objective

§ 4.1
Provide a goal-local regression test for the exact fields W01 and W02 transport and grade.

## Instructions

§ 5.1
Add a complete AR-01 JSON fixture with all required fields, pass it through the same serialization shape used by W01, and invoke the direct blinded grader from test-review-oracle.sh.

## Acceptance criteria

§ 6.1
The regression asserts path, location, summary, evidence, required_correction, and independent survive unchanged and that one consolidated finding matches all three defects.

## Handoff

§ 7.1
Goal 02 consumes the stable envelope and need not reconstruct missing fields.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
