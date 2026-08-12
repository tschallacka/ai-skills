# Step: 03-step-last-button-click

## Ownership

- Goal: `01-button-chain-contract`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `handleButtonClick(event)`
- Subscope: `N/A`

## Objective

§ 4.1
Append exactly one new button below the current last button when that current last button is pressed, ignoring older buttons for append behavior.

## Instructions

§ 5.1
In handleButtonClick(event), determine whether the clicked button is the current last button in the chain. If it is not the current last button, do not append a button. If it is the current last button and fewer than four generated buttons exist, append exactly one new generated button below it and update the generated-button count.

## Acceptance criteria

§ 6.1
Each valid click before completion adds one and only one button after the previous last button. Clicking an older button does not add another button. The initial button is not counted as a generated button.

## Handoff

§ 7.1
W04 can rely on an accurate generated-button count and last-button detection when deciding whether the fourth generated button was pressed.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
