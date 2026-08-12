# Step: 01-step-per-defect-report

## Ownership

- Goal: `04-per-defect-diagnostics`
- Work unit: `W09`
- Type: `source`

## Change target

- File: `benchmark/planning/grade-blinded-run.sh`
- Primary symbol or file scope: `per-defect classification`
- Subscope: `report schema`

## Objective

§ 4.1
Emit per_defect entries with defect_id, classification (true_positive/partial/unresolved/false_positive), candidate finding ids, and the list of failed predicates, while keeping aggregate fields and public redaction.

## Instructions

§ 5.1
In grade-blinded-run.sh, add a partial classification (partial when path/location match but signal or correction fail) and emit per_defect detail into the PRIVATE rows keyed by defect_id. The PUBLIC report exposes only a sanitized projection: per-defect entries with a neutral ordinal index (defect_1..defect_n), the public finding ids considered, the failed predicate list, and the classification, and must never contain seed IDs (SD-NN). Fold partial into counts as not-a-true-positive for the rates and report it as its own count key; preserve all existing aggregate fields.

## Acceptance criteria

§ 6.1
The public report contains sanitized per-defect rows with no SD-NN; private rows carry the defect_id mapping; fresh SD-01 is partial or false_positive with an empty candidate list and a PATH/UNREPORTED predicate note; the aggregate counts dict shape asserted by test-review-oracle.sh is unchanged except the new partial key.

## Handoff

§ 7.1
W10 projects the sanitized rows into metadata/telemetry; W11 proves reproducibility; W12 asserts the public report contains no seed IDs.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
