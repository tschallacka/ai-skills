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
When the safety boundary is lifted, navigate to the future button-chain.html route in a rendered browser. Click the initial visible button, then click each newly appended current last button in order until the fourth generated button is activated. Check that each prior click created exactly one button below the last one and that the fourth generated activation clears the document and displays finished with a visible white border. Do not use console commands, JavaScript evaluation, storage edits, injected events, or direct APIs.

## Acceptance criteria

§ 6.1
Future browser evidence shows the exact route, direct mouse clicks, one-button-per-activation intermediate states, and the final cleared finished state with visible white border. Current proof records this as intentionally untested/excluded.

## Handoff

§ 7.1
W04 stores this flow as US-01 and its run cache; future completion requires actual browser evidence before marking the story passed.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
