# Step: 04-step-authority-regression

## Ownership

- Goal: `02-authority-and-provenance`
- Work unit: `W12`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-review-lifecycle.sh`
- Primary symbol or file scope: `Reviewer B authority fixture`
- Subscope: `N/A`

## Objective

§ 4.1
Provide a goal-local regression test for sole Reviewer B authority and provenance requirements.

## Instructions

§ 5.1
Add approval permutations for A-only, A true/B false, B true, missing B, duplicate B, wrong session, and wrong mode. Assert the selected terminal artifact is exactly Reviewer B when valid.

## Acceptance criteria

§ 6.1
The test asserts A cannot approve, stale or duplicate B evidence fails closed, and all required provenance hashes are present or produce an explicit reason.

## Handoff

§ 7.1
Goal 03 can invoke the adapter with confidence that approval state is authoritative.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
