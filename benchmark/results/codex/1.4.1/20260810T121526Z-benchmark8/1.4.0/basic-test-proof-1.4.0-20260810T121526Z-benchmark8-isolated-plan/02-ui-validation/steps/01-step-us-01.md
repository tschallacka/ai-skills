# Step: 01-step-us-01

## Ownership

- Goal: `02-ui-validation`
- Work unit: `W04`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `US-01 button-chain browser flow`
- Subscope: `N/A`

## Objective

§ 4.1
Verify through direct mouse clicks that initial state, one-button-per-click growth, fourth-generated-button clearing, exact finished text, and visible white border all match the acceptance contract.

## Instructions

§ 5.1
Use a fresh browser context and the US-01 cache. Open the planned local button-chain.html route. Confirm one initial button, click it, and after each DOM update freshly locate the final visible button before the next direct mouse click. Wait for exactly two, three, four, and five buttons with adjacent order after clicks 1–4; on click 5, pressing generated button 4, wait for zero buttons, removal of the button-chain root, and exactly one visible terminal element containing exact lowercase finished on a contrasting background with an explicit 1px solid white border. Record direct mouse interaction, actual waits, button counts/order, exact text, border, URL, and decisive screenshot or observable assertion. Do not use console evaluation, injected events, storage edits, direct API calls, or any shortcut.

## Acceptance criteria

§ 6.1
US-01 is passed only when all five clicks are direct mouse input; counts are exactly 2, 3, 4, and 5 after clicks one through four; generated button 4 is the fifth click target; the fifth click clears the document; and exact lowercase finished on a contrasting background with an explicit 1px solid white border is observable. Otherwise mark the story bug found or untested with evidence.

## Handoff

§ 7.1
On pass, update ui-user-stories.md and ui-story-runs/US-01.md with actual evidence and hand the validated contract to the final analysis report. Before initiative completion, run the tagged normal validator and then validate-plan.sh --complete; on failure, follow bug-register.md and retain the failure trace.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
