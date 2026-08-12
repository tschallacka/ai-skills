# Goal 01 / Step 01: Define the semantic defect manifest

## Ownership

- Goal: `01-semantic-oracle-contract`
- Work unit: `W01`
- Type: `source`

## Change target

- File: `benchmark/planning/seed-blinded-defects.sh`
- Primary symbol or file scope: `semantic manifest validation`
- Subscope: `private metadata`

## Objective

Extend the private blinded-defect schema with semantic location and correction
metadata while keeping seed IDs and keys inaccessible to reviewers.

## Files or areas

`benchmark/planning/seed-blinded-defects.sh` only. W12 owns the contract tests
and fixtures; W01 supplies the source schema behavior they exercise.

## Instructions

Add validated fields for path, location/anchor, expected signal, required
correction, and severity. Preserve final-file hashing when multiple mutations
share a file. Ensure encrypted manifests and private snapshots retain the
metadata and no private field is copied into the reviewer capsule.

## Acceptance criteria

- Invalid or incomplete semantic fields fail closed.
- Three mutations in one file produce one valid final hash and three semantic
  records.
- Reviewer-visible files contain no seed IDs, keys, or semantic manifest.

## Handoff

The encrypted manifest schema is ready for semantic adjudication in W02.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
