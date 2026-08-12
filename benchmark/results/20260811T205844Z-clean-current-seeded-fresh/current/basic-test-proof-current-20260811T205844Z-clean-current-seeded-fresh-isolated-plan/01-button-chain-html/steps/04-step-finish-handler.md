# Step: 04-step-finish-handler

## Ownership

- Goal: `01-button-chain-html`
- Work unit: `W04`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `finishDocument()`
- Subscope: `N/A`

## Objective

§ 4.1
Implement the fourth generated button completion path so activation clears the document and prints lowercase finished with a visible white border.

## Instructions

§ 5.1
Future executor implements finishDocument() so pressing generated button 4 clears all prior document body content and renders one completion element containing exactly finished in lowercase with a visible white border.

## Acceptance criteria

§ 6.1
Click 5 from the initial state, performed on generated button 4, leaves no buttons in the document and renders exact visible text finished with a nonzero visible white border. No other casing, extra text, or remaining chain content is accepted.

## Handoff

§ 7.1
W05 and W06 can assert the five-click sequence and final completion state.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
