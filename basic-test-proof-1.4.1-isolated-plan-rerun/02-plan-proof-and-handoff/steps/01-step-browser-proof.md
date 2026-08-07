# Step: 01-step-browser-proof

## Ownership

- Goal: `02-plan-proof-and-handoff`
- Work unit: `W03`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `browser button-chain flow`
- Subscope: `N/A`

## Objective

§ 4.1
Verify the future rendered flow by pressing the initial/current last button through the fourth generated button and checking the final finished output and white border.

## Instructions

§ 5.1
After the planning-only restriction is lifted and W01, W02, and W07 pass, open button-chain.html in a fresh real browser context without a server if local-file navigation is supported; otherwise use only a user-approved serving method. Execute the cached US-01 sequence with real clicks or keyboard activation on the visible current last button. After each of the first four presses assert exactly one new button appears below; after the fifth assert every button and prior document node is gone and only finished with a visible white border remains. Capture one decisive final screenshot and route evidence.

## Acceptance criteria

§ 6.1
US-01 passes only with five direct UI interactions, exact per-press counts of 2, 3, 4, and 5 total buttons before completion, then zero buttons and only exact finished completion output after press 5, with a visibly white rendered border. Console evaluation, injected events, DOM edits, direct API calls, or inferred source behavior cannot count as evidence.

## Handoff

§ 7.1
W04 records the future evidence and status. A failure triggers the documented bug loop rather than weakening the story or changing code in W03.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
