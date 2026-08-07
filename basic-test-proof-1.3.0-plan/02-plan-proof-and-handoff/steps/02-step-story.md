# Step: 02-step-story

## Ownership

- Goal: `02-plan-proof-and-handoff`
- Work unit: `W04`
- Type: `docs`

## Change target

- File: `ui-user-stories.md`
- Primary symbol or file scope: `US-01`
- Subscope: `N/A`

## Objective

§ 4.1
Record the direct-interaction UI acceptance story, explicit planning-only exclusion, cache, evidence boundary, and related verification unit.

## Instructions

§ 5.1
Record US-01 with the exact future precondition, direct mouse-click sequence, generated-button counting rule, expected final observable state, and explicit user-approved planning-only exclusion. Configure ui-story-runs/US-01.md with the same interaction sequence and an untested result; maintain an empty bugs.md because no browser run occurs.

## Acceptance criteria

§ 6.1
The story has a direct interaction, a related verification unit, a valid cache path with no prohibited shortcuts or placeholders, and evidence that no browser execution was claimed during this proof.

## Handoff

§ 7.1
W05 validates the story/cache structure; a future executor updates the story and cache only after real browser evidence.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
