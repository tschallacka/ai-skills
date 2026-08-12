# Step: 01-step-plan-mutation-protocol

## Ownership

- Goal: `21-plan-mutation-helper-governance`
- Work unit: `W90`
- Type: `docs`

## Change target

- File: `planning/SKILL.md`
- Primary symbol or file scope: `helper-only plan mutation protocol`
- Subscope: `N/A`

## Objective

§ 4.1
Document the rule that durable .plans mutations must use validated helpers.

## Instructions

§ 5.1
Add the helper-only rule to SKILL.md and document that many helper calls should be batched in one temporary executable script under strict mode, followed by one validator run.

## Acceptance criteria

§ 6.1
The skill explicitly prohibits direct durable .plans edits and names the canonical dispatcher and helper commands.

## Handoff

§ 7.1
Hand the protocol to W91 and all future plan workers.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
