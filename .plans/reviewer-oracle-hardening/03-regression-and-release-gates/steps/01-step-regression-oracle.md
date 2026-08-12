# Goal 03 / Step 01: Test semantic oracle regression

## Ownership

- Goal: `03-regression-and-release-gates`
- Work unit: `W07`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-review-oracle.sh`
- Primary symbol or file scope: `semantic oracle contract test`
- Subscope: `consolidated finding cases`

## Objective

Prove that one AR finding can semantically cover three hidden defects.

## Instructions

Test valid one-to-many coverage, partial coverage, ambiguous evidence,
duplicate findings, exact-ID diagnostics, and role rejection. Keep these as
unit/contract cases, not end-to-end substitutes.

## Acceptance criteria

The consolidated finding yields three semantic true positives and exact-ID
diagnostics remain separate.

## Handoff

W08 and W09 add approval and boundary cases; W11 consumes all contract suites.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
