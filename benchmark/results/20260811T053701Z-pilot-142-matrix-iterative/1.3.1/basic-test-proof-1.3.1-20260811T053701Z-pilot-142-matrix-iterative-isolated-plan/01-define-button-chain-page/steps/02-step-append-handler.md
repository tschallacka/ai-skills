# Step: 02-step-append-handler

## Ownership

- Goal: `01-define-button-chain-page`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `appendNextButton()`
- Subscope: `N/A`

## Objective

§ 4.1
Define click handling so only pressing the current last button appends exactly one new button below it.

## Instructions

§ 5.1
In the future implementation, define appendNextButton() so a click on the current last button appends exactly one new button below it.

§ 5.2
Guard earlier buttons after they are no longer last so pressing a non-last button does not append additional buttons.

## Acceptance criteria

§ 6.1
After clicking the current last button before completion, the total button count increases by exactly one and the new button is visually below the previous last button.

§ 6.2
Clicking any button that is not currently last leaves the button count unchanged.

## Handoff

§ 7.1
W03 can rely on generated-button order and the current-last-button rule.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
