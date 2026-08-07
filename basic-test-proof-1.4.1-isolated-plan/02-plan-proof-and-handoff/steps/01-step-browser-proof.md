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
Verify the future rendered flow by clicking the initial and current-last buttons through the fourth generated button and checking the final finished output and white border.

## Instructions

§ 5.1
Future executor only: open button-chain.html in a fresh rendered browser, click the visible initial button, wait for exactly one new current-last button below it, then repeat through generated button four. After pressing generated four, verify the document was cleared and only finished with a visible white border remains. Do not start this flow now.

## Acceptance criteria

§ 6.1
Pass only with real user-facing clicks, exactly one append below after every pre-completion press, completion on the fourth generated button rather than the initial button, cleared prior content, finished text, and visible white border. Current planning status is user-approved exclusion, not browser evidence.

## Handoff

§ 7.1
W04 records the story and exclusion. Future execution must replace the exclusion with decisive rendered evidence unless the user grants a new explicit exclusion.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
