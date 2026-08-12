# Step: 02-step-style-completion

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W02`
- Type: `style`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `.completion-message`
- Subscope: `N/A`

## Objective

§ 4.1
Define the visible white border styling for the finished completion state in button-chain.html.

## Instructions

§ 5.1
In button-chain.html, define .completion-message so the final state visibly surrounds the exact text with a white border. Keep the selector limited to completion output styling and do not change append behavior here.

## Acceptance criteria

§ 6.1
.completion-message produces a visible white border in the final state, while the initial state still does not display finished text.

## Handoff

§ 7.1
W04 can render the finished completion element with this class after clearing the document.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
