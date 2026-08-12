# Goal 01 / Step 04: Test manifest schema and final hashes

## Ownership

- Goal: `01-semantic-oracle-contract`
- Work unit: `W12`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-review-oracle.sh`
- Primary symbol or file scope: `semantic manifest schema test`
- Subscope: `manifest validation`

## Objective

Keep required semantic fields, final multi-mutation hashes, and private metadata
validation covered by a dedicated contract test.

## Instructions

Test missing location, signal, correction, and severity fields; multiple edits
to one file; final hash stability; and encrypted-private-material permissions.

## Acceptance criteria

Invalid manifests fail closed and valid multi-edit manifests preserve all
semantic records and the final defective hash.

## Handoff

W07 consumes this schema contract in consolidated-finding tests.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
