# Goal 02 / Step 05: Regenerate the reviewer projection

## Ownership

- Goal: `02-approval-state-integrity`
- Work unit: `W14`
- Type: `generated`

## Change target

- File: `planning/REVIEWER.md`
- Primary symbol or file scope: `reviewer projection`
- Subscope: `approval evidence`

## Objective

Regenerate the review-scoped contract from the authoritative planning skill.

## Instructions

Run the repository's reviewer-generation command after W05 changes and verify
the projection states Reviewer A/B ownership, boolean approval, evidence fields,
consolidated findings, and non-adoptable false approval.

## Acceptance criteria

The projection hash/source metadata is current and contains no contradictory
approval language.

## Handoff

W15 aligns the benchmark prompt; W09 verifies the projection boundary.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
