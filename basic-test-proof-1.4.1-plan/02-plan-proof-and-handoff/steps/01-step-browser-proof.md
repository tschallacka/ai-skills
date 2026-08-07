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
Verify the future rendered flow by clicking the initial/current last button through the fourth generated button and checking the final finished output and white border.

## Instructions

§ 5.1
Future executor only: open button-chain.html in a fresh rendered browser context, click the visible initial button, wait for the new visible current-last button, and repeat the same direct mouse click through generated button four. Verify after each pre-completion click that exactly one button appears below the prior last button, then verify the final document shows finished with a visible white border. Current proof must not run this browser flow.

## Acceptance criteria

§ 6.1
Pass only when the direct-click sequence reaches generated button four, each earlier click appends exactly one button below the current last button, and completion clears the chain and shows finished with a white border. Planning-time status is excluded by explicit user approval and must not be represented as browser evidence.

## Handoff

§ 7.1
W04 records this story and its exclusion evidence; a future implementation run must replace the exclusion with browser evidence or obtain a new explicit user-approved exclusion.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
