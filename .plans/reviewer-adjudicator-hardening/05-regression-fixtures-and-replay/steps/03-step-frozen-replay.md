# Step: 03-step-frozen-replay

## Ownership

- Goal: `05-regression-fixtures-and-replay`
- Work unit: `W14`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-frozen-replay.sh`
- Primary symbol or file scope: `frozen archive regrade expectations`
- Subscope: `N/A`

## Objective

§ 4.1
Add a deterministic test that regrades the frozen approval.json archives against pilot-blinded-defects.json and pins iterative 3/3 and fresh 2/3 as expectations without editing archives.

## Instructions

§ 5.1
Create test-frozen-replay.sh (a NEW file) that discovers the two frozen Reviewer B approvals by globbing benchmark/results/<run-id>/current/reviewers/*-B-*/plan/approval.json (mirroring select_reviewer_b_approval), reads findings from approved_findings only, regrades them against pilot-blinded-defects.json old/new and expected_signal, and asserts the pinned per-defect classifications: iterative SD-01/02/03 true_positive (3/3), fresh SD-01 false_positive and SD-02/SD-03 true_positive (2/3). No archived file is edited.

## Acceptance criteria

§ 6.1
The test passes deterministically and documents that fresh SD-01 is an honest miss with no candidate finding.

## Handoff

§ 7.1
W15 includes this test in the final gate; if it fails, use the W09 failed-predicate list to diagnose before changing expectations.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
