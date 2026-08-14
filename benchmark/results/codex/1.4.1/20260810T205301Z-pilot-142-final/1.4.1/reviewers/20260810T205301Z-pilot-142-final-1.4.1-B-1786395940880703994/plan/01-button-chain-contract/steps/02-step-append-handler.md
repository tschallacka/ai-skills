# Step: 02-step-append-handler

## Ownership

- Goal: `01-button-chain-contract`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `appendNextButton()`
- Subscope: `N/A`

## Objective

§ 4.1
Define the click handler behavior so only the current last button appends exactly one new button below itself.

## Instructions

§ 5.1
Implement appendNextButton() in button-chain.html. It must respond only when the clicked element is the current last button, increment the generated-button count by one, create exactly one new button, and insert that new button below the clicked last button.

## Acceptance criteria

§ 6.1
After each nonterminal last-button click, the number of visible buttons increases by exactly one, the new button appears below the prior last button, and clicking an older non-last button cannot append another button.

## Handoff

§ 7.1
W03 can use the same generated-button count to detect that the clicked current last button is the fourth generated button.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
