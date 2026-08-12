# Step: 09-step-review-oracle

## Ownership

- Goal: `07-pilot-release`
- Work unit: `W58`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-review-oracle.sh`
- Primary symbol or file scope: `seeded defect oracle test function`
- Subscope: `N/A`

## Objective

§ 4.1
Define a blinded seeded-defect set and calculate true positives, false negatives, independent catches, duplicates, unresolved findings, and accuracy denominators for iterative and fresh-review runs.

## Instructions

§ 5.1
Write `/tmp/<analysis-run-id>/oracle.json` with schema_version, fixture_hash, mode, revision, seeded_defects[], findings[{finding_id,reviewer_session_id,classification}], true_positives, false_positives, duplicates, unresolved, false_negatives, denominators, true_positive_rate, independent_catch_rate, and evidence_paths. W59 consumes only schema-valid oracle.json.

## Acceptance criteria

§ 6.1
The fixture contains at least three stable seeded defect IDs and expected outcomes; both modes produce blinded machine-readable classifications; denominators, duplicate handling, and unresolved findings are identical across modes; missing oracle evidence fails the test.

## Handoff

§ 7.1
Hand off the fixture hash, defect IDs, attribution rules, denominators, and mode-by-mode oracle report to W59 and W38.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
