# Step: 02-step-button-chain-logic

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `appendNextButton(event)`
- Subscope: `N/A`

## Objective

§ 4.1
Add event logic that appends exactly one button below the current last button and clears the document when the fourth generated button is pressed.

## Instructions

§ 5.1
Define appendNextButton(event) in button-chain.html. Track generated buttons as buttons appended after the initial button. On each accepted click, first verify the clicked element is the current last button in the chain. For generated counts one through three, append exactly one new button below the current last button. When the clicked button is the fourth generated button, clear the document body and render only the terminal finished element that W03 styles. Older-button clicks must not append or clear anything.

## Acceptance criteria

§ 6.1
Accepted clicks on the current last button append one and only one button for generated buttons one, two, and three; pressing the fourth generated button clears the document and emits the terminal finished element; clicks on any non-last button have no effect.

## Handoff

§ 7.1
W03 can style the terminal element emitted by appendNextButton(event) using the .finished-state selector. W04 can inspect the generated-button counter and current-last-button guard.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
