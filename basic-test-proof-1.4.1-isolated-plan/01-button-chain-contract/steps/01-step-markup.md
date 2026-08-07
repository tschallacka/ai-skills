# Step: 01-step-markup

## Ownership

- Goal: `01-button-chain-contract`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain`
- Subscope: `N/A`

## Objective

§ 4.1
Create the future root HTML button-chain subtree with exactly one initial button.

## Instructions

§ 5.1
Future execution only: create repository-root button-chain.html with a #button-chain subtree containing exactly one initially visible button. Do not add unrelated controls or dependencies, and do not execute or inspect HTML during this planning proof.

## Acceptance criteria

§ 6.1
The future target is exactly button-chain.html; #button-chain contains exactly one visible button before interaction; no second target is included; this proof creates no HTML.

## Handoff

§ 7.1
W02 may rely on the single initial button inside #button-chain and must attach only the current-last-button behavior.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
