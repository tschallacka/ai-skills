# Step: 06-step-bind-reviewer-session

## Ownership

- Goal: `02-authority-and-provenance`
- Work unit: `W15`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `reviewer_b_session_binding`
- Subscope: `N/A`

## Objective

§ 4.1
Bind the selected approval to the exact Reviewer B lifecycle session and capsule; reject wrong-session, wrong-mode, stale, duplicate, and mismatched artifacts before terminal evidence.

## Instructions

§ 5.1
Compare the selected Reviewer B approval with the launched lifecycle session and capsule manifest. Require exact session ID, capsule ID, current protocol mode, non-stale approval timestamp, and one-to-one approval identity. Reject wrong-session, wrong-mode, stale, duplicate, or cross-capsule artifacts with explicit state reasons before W06 publishes provenance.

## Acceptance criteria

§ 6.1
Only one schema-valid, current Reviewer B approval can pass binding. Every rejected permutation is non-terminal and records a deterministic reason; a passing approval exposes exact IDs and hashes for W06.

## Handoff

§ 7.1
W06 can publish provenance for one attributable Reviewer B session/capsule and cannot accidentally promote another reviewer’s or another run’s approval.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
