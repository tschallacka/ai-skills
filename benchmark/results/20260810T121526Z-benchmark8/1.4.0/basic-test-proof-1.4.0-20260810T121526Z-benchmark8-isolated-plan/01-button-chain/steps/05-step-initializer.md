# Step: 05-step-initializer

## Ownership

- Goal: `01-button-chain`
- Work unit: `W06`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `button-chain initializer`
- Subscope: `N/A`

## Objective

§ 4.1
Initialize #button-chain-root with one initial button, the generated-button counter state, and the current-last click-handler attachment point without implementing the click branch.

## Instructions

§ 5.1
In the future button-chain.html, define only the button-chain initializer: create #button-chain-root with one initial button, initialize generated-button counter state, and establish the handler attachment point. Do not implement the click branch or terminal rendering here.

## Acceptance criteria

§ 6.1
The future page initializes exactly one visible button, a deterministic generated-button counter, and a current-last handler attachment point; no generated button or finished terminal state exists before interaction.

## Handoff

§ 7.1
W03 consumes the initialized root, initial control, counter, and handler attachment point; W05 checks this separation before W04.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
