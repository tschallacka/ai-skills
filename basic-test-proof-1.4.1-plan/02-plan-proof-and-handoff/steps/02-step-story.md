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
Maintain US-01 in ui-user-stories.md with the direct click sequence, expected visible results, related W03 verification unit, and the explicit user-approved reason that browser execution is prohibited in this planning proof. Keep ui-story-runs/US-01.md aligned and preserve the empty bugs.md register.

## Acceptance criteria

§ 6.1
US-01 is bounded, names a real mouse-click interaction, maps to W03, has a complete cache with no prohibited shortcut, records planning-time exclusion with user approval, and leaves no open bug row.

## Handoff

§ 7.1
W05 can validate the UI artifacts and the final handoff can identify exactly what remains deferred: the future browser run and HTML implementation.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
