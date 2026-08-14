# Step: 02-step-append-handler

## Ownership

- Goal: `01-button-chain-behavior`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `handleButtonClick(event)`
- Subscope: `append-last-button branch`

## Objective

§ 4.1
Append exactly one new button below the current last button only when that current last button is pressed.

## Instructions

§ 5.1
Implement the append branch inside `handleButtonClick(event)`. If the clicked element is not the current last button in `#button-chain-root`, return without changing the DOM. If it is the current last button and completion is not due, create exactly one new button and append it below the current last button.

## Acceptance criteria

§ 6.1
Each accepted click increases the button count by one and places the new button after the previous last button. Clicking an earlier button after a later button exists does not append another button.

## Handoff

§ 7.1
W03 can rely on generated buttons being counted and appended one at a time through `handleButtonClick(event)`.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
