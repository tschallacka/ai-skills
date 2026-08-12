# Step: 06-step-fourth-generated-handler-branch

## Ownership

- Goal: `01-button-chain-contract`
- Work unit: `W07`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `handleButtonClick(event)`
- Subscope: `fourth-generated terminal branch`

## Objective

§ 4.1
Add the handler branch that recognizes a click on the fourth generated current-last button and delegates to renderFinishedState() instead of appending another button.

## Instructions

§ 5.1
In the future implementation, update only the fourth-generated terminal branch inside handleButtonClick(event).

§ 5.2
When the clicked element is the current last button and is the fourth generated button, call renderFinishedState() and do not append a fifth generated button.

## Acceptance criteria

§ 6.1
Clicking the fourth generated current-last button delegates to renderFinishedState() instead of appending another button.

§ 6.2
The non-terminal append behavior from W02 remains unchanged.

## Handoff

§ 7.1
W05 can exercise five direct clicks: four append-producing clicks followed by the fourth-generated terminal click.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
