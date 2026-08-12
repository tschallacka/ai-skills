# Step: 01-step-root-markup

## Ownership

- Goal: `01-button-chain-contract`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain-root`
- Subscope: `N/A`

## Objective

§ 4.1
Create the root document body subtree containing exactly one initial button and no completion message at load.

## Instructions

§ 5.1
Create button-chain.html as a standalone document. In the #button-chain-root subtree, render exactly one initial button at load, give it a stable accessible name, and do not render finished text or hidden extra buttons.

## Acceptance criteria

§ 6.1
A reviewer can inspect the future file and identify one initial button inside #button-chain-root, no other button elements, and no completion message in the initial state.

## Handoff

§ 7.1
W02 can attach behavior to the initial button and any generated current-last button without changing the root markup target.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
