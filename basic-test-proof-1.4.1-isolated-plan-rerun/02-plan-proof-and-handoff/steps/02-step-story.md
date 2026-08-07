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
Maintain US-01 and ui-story-runs/US-01.md through the UI helpers. For this planning-only proof, record status excluded solely because the user explicitly forbids opening or testing HTML and starting a browser; state that no UI evidence was collected. Preserve the configured five-input future sequence, readiness conditions, expected results, W01-W03 mapping, and empty bug register.

## Acceptance criteria

§ 6.1
US-01 has one bounded direct-interaction contract, maps to W01-W03, contains no prohibited state shortcut, records the explicit user-approved planning-time exclusion, and does not claim passed or browser evidence. Its cache remains untested and ready for a future run.

## Handoff

§ 7.1
W05 may treat the UI boundary as honestly documented but must not treat US-01 as executed or passed.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
