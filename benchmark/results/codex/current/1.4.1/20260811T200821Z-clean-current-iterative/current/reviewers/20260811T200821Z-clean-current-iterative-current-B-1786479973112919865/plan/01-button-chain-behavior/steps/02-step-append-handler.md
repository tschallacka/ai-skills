# Step: 02-step-append-handler

## Ownership

- Goal: `01-button-chain-behavior`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `appendNextButton`
- Subscope: `N/A`

## Objective

§ 4.1
Add the click handler behavior that appends exactly one button below the current last button only when that current last button is pressed.

## Instructions

§ 5.1
Implement appendNextButton so a click on the current last button appends exactly one new button below it and updates last-button ownership to the appended button.

## Acceptance criteria

§ 6.1
Each accepted current-last click increases the button count by one; clicking an older button after a new button exists appends nothing.

## Handoff

§ 7.1
W03 can consume the generated-button count and last-button state.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
