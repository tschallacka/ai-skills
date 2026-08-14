# Step: 01-step-initial-markup

## Ownership

- Goal: `01-define-button-chain-page`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain-root`
- Subscope: `N/A`

## Objective

§ 4.1
Create the initial document body with exactly one starting button and a vertical container for generated buttons.

## Instructions

§ 5.1
In the future implementation, create button-chain.html as a standalone document with a body-level button-chain-root subtree containing exactly one initial button and no generated buttons at load.

§ 5.2
Place generated buttons in a vertical order below the preceding last button without adding unrelated controls or explanatory UI.

## Acceptance criteria

§ 6.1
A fresh load of the future file shows exactly one button and no finished message.

## Handoff

§ 7.1
W02 can attach click behavior to the initial button and generated button container.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
