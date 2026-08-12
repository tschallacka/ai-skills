# Step: 05-step-independent-grader

## Ownership

- Goal: `20-blinded-seeded-defect-oracle`
- Work unit: `W98`
- Type: `source`

## Change target

- File: `benchmark/planning/grade-blinded-run.sh`
- Primary symbol or file scope: `Independent blinded-run grader`
- Subscope: `N/A`

## Objective

§ 4.1
Decrypt and classify retained target evidence in a separate oracle process

## Instructions

§ 5.1
Keep grading in the independent oracle process: decrypt only after terminal evidence, validate map and target hashes, classify findings, and emit counts without defect paths or secret material.

## Acceptance criteria

§ 6.1
The grader rejects missing role, key, map, terminal evidence, and hash mismatches; a valid run produces a mode-independent report with no plaintext map or key.

## Handoff

§ 7.1
W100 verifies the standalone grader through the blinded protocol test.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
