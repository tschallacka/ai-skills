# Step: 02-step-append-current-last-button

## Ownership

- Goal: `01-button-chain-html`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `appendNextButton()`
- Subscope: `N/A`

## Objective

§ 4.1
Implement the click path for the current last button so one and only one new button is appended below it.

## Instructions

§ 5.1
Implement appendNextButton() in button-chain.html. Attach click behavior so only the current last button in the rendered chain is allowed to append the next generated button below itself.

§ 5.2
Maintain generated-button order and count explicitly. On each non-terminal valid click, append exactly one new button and do not mutate completion state. Ignore clicks on any button that is no longer current last.

## Acceptance criteria

§ 6.1
Clicking the initial current-last button creates exactly generated button 1 below it.

§ 6.2
Clicking generated buttons 1, 2, and 3 while each is current last creates exactly one next generated button below the previous last button.

§ 6.3
Clicking an earlier, non-last button after another button exists does not append any button.

## Handoff

§ 7.1
W03 can rely on a generated-button count and a current-last guard that identify generated button 4 as the terminal control.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
