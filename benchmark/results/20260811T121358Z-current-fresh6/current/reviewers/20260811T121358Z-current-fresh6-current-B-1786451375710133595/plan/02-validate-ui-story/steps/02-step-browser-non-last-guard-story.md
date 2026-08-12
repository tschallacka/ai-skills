# Step: 02-step-browser-non-last-guard-story

## Ownership

- Goal: `02-validate-ui-story`
- Work unit: `W07`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `US-02 non-last-button guard flow`
- Subscope: `N/A`

## Objective

§ 4.1
Run the future browser story that verifies clicking a no-longer-last button does not append another button.

## Instructions

§ 5.1
When executing later, open the local button-chain.html in a browser and create at least two generated buttons through normal current-last-button clicks.

§ 5.2
Click a visible button that is no longer the current last button and observe that the button count and order do not change.

## Acceptance criteria

§ 6.1
The story passes only if the non-last click appends zero buttons and the current last button remains unchanged.

## Handoff

§ 7.1
If the guard story fails, record a bugs.md row and create investigation and fix goals before retesting.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
