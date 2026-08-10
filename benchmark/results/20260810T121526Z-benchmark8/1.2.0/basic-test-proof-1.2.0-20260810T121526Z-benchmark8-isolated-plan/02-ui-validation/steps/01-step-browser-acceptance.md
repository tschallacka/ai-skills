# Step 01: browser acceptance

## Ownership
- Goal: `02-ui-validation`
- Work unit: `W05`
- Type: `verification`

## Change target
- File: `N/A`
- Primary symbol or file scope: `US-01 button-chain acceptance flow`
- Subscope: `N/A`

## Objective
Run the final UI story against the future workspace-root `button-chain.html`.

## Instructions
1. Open the workspace-root local file after future implementation and click accessible `Button 0`, `Button 1`, `Button 2`, `Button 3`, then `Button 4`, waiting for the named next target after each append.

## Acceptance criteria
- The first four clicks each add exactly one button below the prior last button; the fifth click clears the document and leaves exact lowercase `finished` with a visible white border.

## Handoff
- W11, W12, and W13 record the separate story, cache, and bug artifacts.

## Atomicity check
- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
