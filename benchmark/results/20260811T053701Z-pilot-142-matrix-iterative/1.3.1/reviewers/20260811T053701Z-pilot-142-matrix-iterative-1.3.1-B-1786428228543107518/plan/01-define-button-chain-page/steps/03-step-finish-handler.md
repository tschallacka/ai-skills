# Step: 03-step-finish-handler

## Ownership

- Goal: `01-define-button-chain-page`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `finishOnFourthGeneratedButton()`
- Subscope: `N/A`

## Objective

§ 4.1
Define the fourth generated button click path so it clears the document and renders exact lowercase text finished.

## Instructions

§ 5.1
In the future implementation, define finishOnFourthGeneratedButton() so the fourth generated button is the completion trigger.

§ 5.2
When that fourth generated button is clicked as the current last button, clear the document body and render only the completion state containing exact lowercase text finished.

## Acceptance criteria

§ 6.1
The completion click removes the button chain and no buttons remain visible.

§ 6.2
The completion text is exactly finished, all lowercase, with no extra visible words.

## Handoff

§ 7.1
W04 can style the completion element produced by W03.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
