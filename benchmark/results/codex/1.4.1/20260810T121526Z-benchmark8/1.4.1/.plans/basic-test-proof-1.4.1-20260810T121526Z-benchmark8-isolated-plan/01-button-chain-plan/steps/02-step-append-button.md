# Step: 02-step-append-button

## Ownership

- Goal: `01-button-chain-plan`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `appendButton()`
- Subscope: `click callback`

## Objective

§ 4.1
Append exactly one button after the current last button, preserving vertical DOM order.

## Instructions

§ 5.1
Implement only appendButton() in button-chain.html: on activation of the current last button, create exactly one new button, insert it immediately below the activating button, and make the new last button the next direct-input target.

## Acceptance criteria

§ 6.1
Each activation before the terminal threshold increases the button count by exactly one, maintains top-to-bottom order, and does not append duplicates or siblings elsewhere.

## Handoff

§ 7.1
W03 can use the generated activation count and current-last-button contract to define the fourth generated branch.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
