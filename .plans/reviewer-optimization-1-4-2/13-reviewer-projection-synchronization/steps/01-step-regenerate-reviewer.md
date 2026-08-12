# Step: 01-step-regenerate-reviewer

## Ownership

- Goal: `13-reviewer-projection-synchronization`
- Work unit: `W72`
- Type: `source`

## Change target

- File: `planning/REVIEWER.md`
- Primary symbol or file scope: `generated reviewer projection`
- Subscope: `N/A`

## Objective

Regenerate the reviewer projection from the authoritative planning skill.

## Instructions

Use `planning/scripts/generate-reviewer.sh` and do not hand-edit generated
content. Preserve the current profile version and update the source hash.

## Acceptance criteria

The projection contains the current source hash and all required reviewer
sections.

## Handoff

W73 verifies reproducibility and required content.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
