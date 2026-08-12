# Goal 02 / Step 02: Clarify Reviewer B evidence contract

## Ownership

- Goal: `02-approval-state-integrity`
- Work unit: `W05`
- Type: `docs`

## Change target

- File: `planning/SKILL.md`
- Primary symbol or file scope: `reviewer contract`
- Subscope: `Reviewer B approval evidence`

## Objective

Align the planning reviewer contract and generated prompt with the state model:
Reviewer B may reject a plan while still producing terminal independent evidence.

## Files or areas

`planning/SKILL.md` only. W14 owns generated `planning/REVIEWER.md`; W15 owns
`benchmark/planning/worker-prompt.md`.

## Instructions

Clarify that Reviewer A cannot approve; Reviewer B must issue a boolean plan
approval; false is valid terminal evidence but not an adoption pass. Require
findings to include stable AR IDs, precise file/section evidence, impact, and
required correction so an independent oracle can adjudicate them.

## Acceptance criteria

- Contract language has no contradiction between “final approval” and “may
  never issue overall approval.”
- The source contract defines required fields for generated projections and
  prompts without directly editing those outputs.
- Consolidated findings remain allowed and explicitly supported.

## Handoff

Goal 03 uses the aligned contract to build prompt and regression assertions.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
