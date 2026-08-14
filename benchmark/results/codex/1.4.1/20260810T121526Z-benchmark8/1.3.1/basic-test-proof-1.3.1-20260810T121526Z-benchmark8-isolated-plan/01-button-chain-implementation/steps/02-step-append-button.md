# Step: 02-step-append-button

## Ownership

- Goal: `01-button-chain-implementation`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `appendButton()`
- Subscope: `generated-button creation branch`

## Objective

§ 4.1
Define appendButton() so an activation of the current last button creates exactly one new button, inserts it immediately below that button, labels it as the next generated button, and makes it the current last target until generated button 4 is pressed.

## Instructions

§ 5.1
Define appendButton() so an activation of the current last button creates exactly one new button, inserts it immediately below that button, labels generated buttons 1–4, and makes the new button the current last target.

## Acceptance criteria

§ 6.1
The initial click and the next three current-last clicks produce generated buttons 1, 2, 3, and 4 respectively; each intermediate state has exactly one more visible button than the previous state.

## Handoff

§ 7.1
W03 can consume a deterministic generated-activation count and the current last target.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
