# Step: 06-step-browser-story

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W06`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `US-01 browser flow`
- Subscope: `N/A`

## Objective

§ 4.1
Using direct clicks, verify the initial button, four generated-button progression, no extra append on completion, document clear, exact finished text, and visible white border.

## Instructions

§ 5.1
After future implementation, run US-01 through the browser using direct mouse clicks only. Click the initial button, then generated buttons 1, 2, 3, and 4 as each becomes the current last visible button. Do not use console, injected JavaScript, storage edits, or direct API calls.

## Acceptance criteria

§ 6.1
US-01 passes only when the first four clicks each append exactly one lower button and the fifth click clears the document to exact text finished with a visible white border. Record route, click sequence, and observed result in ui-story-runs/US-01.md.

## Handoff

§ 7.1
W07 can rely on the browser evidence or, if it fails, on the bug register entry and new investigation/fix scope.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
