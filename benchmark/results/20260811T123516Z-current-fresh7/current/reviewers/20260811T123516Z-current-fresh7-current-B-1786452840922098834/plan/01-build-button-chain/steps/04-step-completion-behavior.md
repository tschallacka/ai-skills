# Step: 04-step-completion-behavior

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W04`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `completeDocument()`
- Subscope: `N/A`

## Objective

§ 4.1
When the fourth generated button is pressed, clear the document and render only the finished completion state with the visible white border.

## Instructions

§ 5.1
Implement completeDocument() and the fourth-generated-button branch so pressing generated button 4 clears the document content and renders the completion state.

§ 5.2
The completion state must contain the exact lowercase text finished and the element carrying that text must use the visible white border style from W02.

## Acceptance criteria

§ 6.1
Source review shows the completion branch is tied to generated button number 4, not the initial button, and that the prior button chain is removed before rendering finished.

## Handoff

§ 7.1
W05 can inspect the completed source for the full contract and W06 can verify it through browser clicks.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
