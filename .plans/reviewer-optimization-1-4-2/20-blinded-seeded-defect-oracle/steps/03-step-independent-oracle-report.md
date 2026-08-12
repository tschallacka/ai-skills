# Step: 03-step-independent-oracle-report

## Ownership

- Goal: `20-blinded-seeded-defect-oracle`
- Work unit: `W88`
- Type: `source`

## Change target

- File: `benchmark/planning/review-oracle.sh`
- Primary symbol or file scope: `independent blinded-run grading`
- Subscope: `N/A`

## Objective

Decrypt the hidden mapping only after target completion and write an auditable
machine-readable oracle report.

## Instructions

Verify seeder/target/oracle role separation, encrypted-manifest and input
hashes, terminal lifecycle evidence, and report schema. Classify true
positives, false negatives, false positives, duplicates, unresolved findings,
and independent catches for iterative and fresh-review runs. Never publish the
key or plaintext mapping.

## Acceptance criteria

The report is independently produced, hash-linked to the run, complete for all
seeded defects, and rejected when evidence or role separation is missing.

## Handoff

Hand accepted oracle reports to W38/W59 and the analyzer; hand rejected reports
to the release blocker record.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
