# Step: 04-step-finish-behavior

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W04`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `finishOnFourthGeneratedButton()`
- Subscope: `N/A`

## Objective

§ 4.1
Add completion behavior so pressing the fourth generated button clears the document and renders exactly finished with the visible white border.

## Instructions

§ 5.1
Implement finishOnFourthGeneratedButton() so the fourth generated button is appended as part of the chain and, when that fourth generated button itself is pressed, the document content is cleared and replaced with a completion element containing exactly finished in lowercase using .completion-message.

## Acceptance criteria

§ 6.1
Pressing generated buttons one through three continues the append chain. Pressing the fourth generated button clears all buttons and leaves only the exact lowercase text finished inside the visible white border.

## Handoff

§ 7.1
W05 receives a complete button-chain.html ready for browser story verification.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
