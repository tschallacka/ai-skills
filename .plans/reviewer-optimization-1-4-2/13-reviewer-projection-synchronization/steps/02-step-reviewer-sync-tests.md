# Step: 02-step-reviewer-sync-tests

## Ownership

- Goal: `13-reviewer-projection-synchronization`
- Work unit: `W73`
- Type: `test`

## Change target

- File: `planning/tests/test-reviewer-projection.sh`
- Primary symbol or file scope: `reviewer projection consistency test`
- Subscope: `N/A`

## Objective

Fail when the generated reviewer projection is stale or missing required
reviewer-visible protocol sections.

## Instructions

Test the source hash, required sections, and deterministic regeneration without
accepting hand-edited or stale output.

## Acceptance criteria

The test fails on a changed source hash or missing required section and passes
for the current generated projection.

## Handoff

Hand off the consistency check to review lifecycle validation.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
