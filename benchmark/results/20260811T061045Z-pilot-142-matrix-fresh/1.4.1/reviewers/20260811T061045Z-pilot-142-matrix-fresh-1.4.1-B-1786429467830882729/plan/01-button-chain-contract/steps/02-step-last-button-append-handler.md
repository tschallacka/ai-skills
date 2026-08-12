# Step: 02-step-last-button-append-handler

## Ownership

- Goal: `01-button-chain-contract`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `handleButtonClick(event)`
- Subscope: `non-terminal append branch`

## Objective

§ 4.1
Add click handling so only pressing the current last button appends exactly one new button below it.

## Instructions

§ 5.1
Add the non-terminal append branch of handleButtonClick(event) inside button-chain.html. It must determine whether the clicked button is the current last chain button before changing the document.

§ 5.2
When the current last button is clicked and the terminal branch does not apply, append exactly one new generated button below it and make that new button the current last button.

## Acceptance criteria

§ 6.1
Clicking the current last button before generated 4 is pressed appends exactly one button below it.

§ 6.2
Clicking an earlier non-last button does not append any button or otherwise change the chain.

## Handoff

§ 7.1
W04 can use the generated-button count maintained by this handler to recognize the fourth generated button.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
