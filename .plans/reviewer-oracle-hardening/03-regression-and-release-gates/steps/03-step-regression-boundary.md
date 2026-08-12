# Goal 03 / Step 03: Test blinding and redaction boundaries

## Ownership

- Goal: `03-regression-and-release-gates`
- Work unit: `W09`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-safeguards.sh`
- Primary symbol or file scope: `blinding boundary test`
- Subscope: `private/public redaction`

## Objective

Prove private seed material cannot leak to reviewers or published reports.

## Instructions

Run the exact capsule-local environment override from plan §8.9. Assert that
reviewer capsules contain neither keys nor decrypted manifests, candidate
envelopes contain only the §8.10 allowed fields, and published archives contain
no seed IDs, mutation keys, private-root paths, defect IDs, or mutation strings.
Assert only the independent oracle can grade private material and that private
paths are transformed to `<private>`.

## Acceptance criteria

All negative-access, environment, artifact-boundary, and redaction checks fail
closed with explicit evidence.

## Handoff

W11 includes the boundary test in the current-protocol archive gate.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
