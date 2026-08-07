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
Maintain US-01 with its real mouse-click sequence, expected visible result, W03 mapping, and explicit user-approved reason that this proof cannot create, open, serve, or test HTML or start a browser. Keep its helper-created cache aligned and bugs.md empty unless a future run finds a bug.

## Acceptance criteria

§ 6.1
US-01 is bounded, includes direct mouse clicks, maps to W03, has a complete readiness cache with no shortcut, records exclusion without fabricated evidence, and has no open bug.

## Handoff

§ 7.1
W05 may rely on structurally complete UI artifacts and an exact list of deferred execution: HTML implementation, contract proof, and browser run.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
