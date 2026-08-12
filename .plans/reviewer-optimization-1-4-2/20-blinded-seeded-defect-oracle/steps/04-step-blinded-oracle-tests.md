# Step: 04-step-blinded-oracle-tests

## Ownership

- Goal: `20-blinded-seeded-defect-oracle`
- Work unit: `W89`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-review-oracle.sh`
- Primary symbol or file scope: `blinded seeded-defect protocol fixtures`
- Subscope: `N/A`

## Objective

Test the complete encrypted seeding, role separation, target isolation, and
independent grading lifecycle.

## Instructions

Add fixtures for encrypted-map creation, missing or exposed key/map, role
identity collision, hash mismatch, incomplete target evidence, complete
iterative/fresh classifications, cleanup, and report schema validation.

## Acceptance criteria

Only a complete independently graded run is accepted; every secrecy,
provenance, or lifecycle failure produces a rejection/blocker without inferred
classification.

## Handoff

Hand genuine oracle evidence to W38/W59 and the final code-to-plan review.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
