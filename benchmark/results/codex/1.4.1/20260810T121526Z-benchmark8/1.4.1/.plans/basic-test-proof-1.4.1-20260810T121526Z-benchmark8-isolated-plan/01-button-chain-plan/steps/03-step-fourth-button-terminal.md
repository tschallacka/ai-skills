# Step: 03-step-fourth-button-terminal

## Ownership

- Goal: `01-button-chain-plan`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `handleButtonClick()`
- Subscope: `fourth-generated-button branch`

## Objective

§ 4.1
Count generated-button activations and clear the document on the fourth generated button.

## Instructions

§ 5.1
Implement only the fourth-generated-button branch in handleButtonClick(): count generated-button activations, and on the fourth generated button clear the document before creating the completion state; do not alter the pre-threshold append rule.

## Acceptance criteria

§ 6.1
After the fourth generated button is activated, the prior button chain is gone, no button remains, and the document contains the completion-state element for W04.

## Handoff

§ 7.1
W04 can style the named completion-message element and W05 can observe the terminal state without relying on hidden state.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
