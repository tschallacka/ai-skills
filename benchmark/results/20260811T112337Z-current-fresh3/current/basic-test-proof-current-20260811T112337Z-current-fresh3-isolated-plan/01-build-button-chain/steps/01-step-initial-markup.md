# Step: 01-step-initial-markup

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain-root`
- Subscope: `N/A`

## Objective

§ 4.1
Create the standalone document structure with one initial button and no generated buttons in the initial DOM.

## Instructions

§ 5.1
During future execution, create button-chain.html as a standalone HTML document whose body contains one named root subtree #button-chain-root and exactly one visible initial button. Do not include generated buttons in the initial DOM.

## Acceptance criteria

§ 6.1
A fresh browser load of the future file shows exactly one button before interaction, and the DOM root for the chain is #button-chain-root.

## Handoff

§ 7.1
W02 can rely on #button-chain-root existing and containing the initial button as the first current last button.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
