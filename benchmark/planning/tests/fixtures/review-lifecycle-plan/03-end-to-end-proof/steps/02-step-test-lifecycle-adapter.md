# Step: 02-step-test-lifecycle-adapter

## Ownership

- Goal: `03-end-to-end-proof`
- Work unit: `W09`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-review-lifecycle.sh`
- Primary symbol or file scope: `approval adapter assertion group`
- Subscope: `N/A`

## Objective

§ 4.1
Test lossless field preservation, malformed-envelope rejection, Reviewer A authority prohibition, sole Reviewer B selection, and explicit state reasons.

## Instructions

§ 5.1
Add isolated fixtures or shell assertions for field-preserving serialization, malformed approval, A-only approval, A unauthorized approval, B-only approval, duplicate B, and provenance omission. For malformed findings assert `schema_status: malformed`, `malformed_findings` reason codes, `malformed_count`, `REVIEW_FINDING_SCHEMA_INVALID`, and adoption false. Keep each failure explicit and fail closed.

## Acceptance criteria

§ 6.1
The lifecycle test fails if a complete finding is reduced to an ID, if malformed evidence lacks its reason/count/state contract, if A can approve, if B is not uniquely selected, or if missing provenance is silently accepted.

## Handoff

§ 7.1
W10 can rely on contract coverage before spending resources on full controls.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
