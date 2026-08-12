# Step: 01-step-browser-story

## Ownership

- Goal: `02-verify-button-chain`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `US-01 browser flow`
- Subscope: `N/A`

## Objective

§ 4.1
Run the browser user story that clicks through the current-last-button chain and verifies completion output.

## Instructions

§ 5.1
Open the implemented button-chain.html in a browser. Use real mouse clicks on the visible current last button: initial button, generated button 1, generated button 2, generated button 3, then generated button 4. Record each click and visible result in ui-story-runs/US-01.md.

## Acceptance criteria

§ 6.1
US-01 passes only when the initial page shows one button, each pre-final click appends exactly one button below the current last button, the fourth generated button click clears the document, and the final visible content is exactly finished with a visible white border.

## Handoff

§ 7.1
If the story fails, update bugs.md and add investigation/fix work units before approval. If it passes, W06 can perform the final artifact audit. This step starts only after W07 readiness proof passes.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
