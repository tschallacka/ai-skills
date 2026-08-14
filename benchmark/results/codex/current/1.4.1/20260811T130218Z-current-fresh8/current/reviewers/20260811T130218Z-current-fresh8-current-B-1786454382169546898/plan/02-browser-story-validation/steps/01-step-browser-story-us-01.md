# Step: 01-step-browser-story-us-01

## Ownership

- Goal: `02-browser-story-validation`
- Work unit: `W06`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `US-01 browser click flow`
- Subscope: `N/A`

## Objective

§ 4.1
Open the implemented file in a browser and use direct clicks on visible buttons from the initial state through the fourth generated button, confirming completion evidence and no unresolved bug rows.

## Instructions

§ 5.1
After implementation only, open the local button-chain.html in a browser and execute ui-story-runs/US-01.md exactly through visible mouse clicks or taps.

§ 5.2
Record the route or file URL, observed button counts after each append click, final finished text, visible white border contrast evidence, and any console or rendering errors relevant to a failure.

§ 5.3
If any expected result fails, mark US-01 as bug found and populate bugs.md with reproduction/evidence, severity, investigation goal, fix goal, retest story, and status before adding the new durable plan work units.

§ 5.4
The bug loop must update plan-description.md, work-unit-inventory.md, progress trackers, ui-user-stories.md, ui-story-runs/US-01.md, bugs.md, testing companions, and rerun validation before execution continues; severe blockers restart testing from US-01 after the fix.

## Acceptance criteria

§ 6.1
US-01 is marked passed only after direct browser clicks produce the final finished state with a visible white border against a contrasting state.

§ 6.2
bugs.md has no unresolved rows at completion, and the story cache contains actual evidence rather than planned-only placeholders.

§ 6.3
Any severe blocker discovered during story execution has completed investigation/fix/retest work units and a restarted US-01 run before final approval.

## Handoff

§ 7.1
Initiative handoff is ready when W06 evidence confirms the full user story and no open bug loop remains.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
