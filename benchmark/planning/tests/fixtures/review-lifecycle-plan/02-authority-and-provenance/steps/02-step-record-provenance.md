# Step: 02-step-record-provenance

## Ownership

- Goal: `02-authority-and-provenance`
- Work unit: `W06`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `protocol-metadata publication block`
- Subscope: `N/A`

## Objective

§ 4.1
Record selected Reviewer B session/capsule, source and defective plan hashes, oracle target snapshot hash, approval hash, and transcript hash with the published decision.

## Instructions

§ 5.1
Publish hashes and identities for source plan, defective plan, oracle target snapshot, selected B capsule/session, approval artifact, transcript, and lifecycle handoff. Link each field to a retained archive path.

## Acceptance criteria

§ 6.1
An independent auditor can reproduce which reviewer and seeded snapshot were graded and detect any hash mismatch; missing provenance fails publication.

## Handoff

§ 7.1
W07 can require the analyzer to report provenance and transformation outcomes distinctly.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
