# Step: 01-step-create-initial-markup

## Ownership

- Goal: `01-create-button-chain-file`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain-root`
- Subscope: `N/A`

## Objective

§ 4.1
Create a valid HTML document containing exactly one initial button and no pre-rendered generated buttons.

## Instructions

§ 5.1
Create button-chain.html as a standalone HTML document. In the document body, render exactly one initial button and no generated buttons. Use stable text or accessible labeling so the current last button is identifiable in browser verification.

## Acceptance criteria

§ 6.1
A fresh load of button-chain.html displays one and only one button before any interaction. No completion message is present initially.

## Handoff

§ 7.1
W02 can attach behavior to the initial button and append generated buttons beneath the existing last button.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
