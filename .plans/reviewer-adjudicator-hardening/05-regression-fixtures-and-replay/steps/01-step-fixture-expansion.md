# Step: 01-step-fixture-expansion

## Ownership

- Goal: `05-regression-fixtures-and-replay`
- Work unit: `W12`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-review-oracle.sh`
- Primary symbol or file scope: `fixtures for multi-path/prose/hyphen/paraphrase/section`
- Subscope: `N/A`

## Objective

§ 4.1
Extend oracle fixtures to cover multi-file path, prose location, section/sec/S variants, hyphenated signal, paraphrase signal, and mutated-conflict positive and negative cases.

## Instructions

§ 5.1
Extend tests/fixtures and test-review-oracle.sh with the natural reviewer shapes: consolidated multi-file path, prose location, section/sec/S variants, hyphenated signal, paraphrase signal, and ordinal/digit forms. Add one positive and one negative case per rule, including a negative multi-path finding naming plan-description.md but about an unrelated defect (stays false_positive) and a mutated-conflict negative (bare echo of one value).

## Acceptance criteria

§ 6.1
Each fixture yields the expected classification; the existing single-file/S-location fixture and the counts dict assertions still pass.

## Handoff

§ 7.1
W14 consumes the fixture expectations and the frozen replay.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
