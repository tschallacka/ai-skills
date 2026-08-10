# Step: 01-step-markup

## Ownership

- Goal: `01-button-chain`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain-root`
- Subscope: `N/A`

## Objective

§ 4.1
Define the single initial button and its containing DOM subtree in the future button-chain.html document.

## Instructions

§ 5.1
In the future button-chain.html, define #button-chain-root with exactly one initially rendered, user-activatable button and stable visible semantics for the story to target. The current target rule is the final visible button in this root; each appended button must remain the immediate next button below the prior target in the same root. Do not add generated buttons or terminal content in this markup step.

## Acceptance criteria

§ 6.1
A fresh page starts with one visible button inside #button-chain-root and no finished message.

## Handoff

§ 7.1
W03 can attach its current-last-button behavior to the one initial control; W04 can locate it by its visible user-facing semantics.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
