# Step: 01-step-initial-button-markup

## Ownership

- Goal: `01-build-contract`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain-root`
- Subscope: `N/A`

## Objective

§ 4.1
Create the standalone document body subtree with exactly one initial button visible at load.

## Instructions

§ 5.1
When executing later, create button-chain.html as a standalone HTML document and define #button-chain-root as the only initial interactive subtree.

§ 5.2
Place exactly one initial button in #button-chain-root. Do not add generated buttons in initial markup.

## Acceptance criteria

§ 6.1
A source review can identify one initial button and no generated buttons before any click.

## Handoff

§ 7.1
W02 can attach behavior to the initial button and the shared #button-chain-root container.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
