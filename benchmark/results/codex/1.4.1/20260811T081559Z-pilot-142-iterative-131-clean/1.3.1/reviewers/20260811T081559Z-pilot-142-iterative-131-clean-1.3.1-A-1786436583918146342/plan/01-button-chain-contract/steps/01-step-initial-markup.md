# Step: 01-step-initial-markup

## Ownership

- Goal: `01-button-chain-contract`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain-app`
- Subscope: `N/A`

## Objective

§ 4.1
Create the document subtree with one initial button and no generated buttons present at load.

## Instructions

§ 5.1
Create button-chain.html with a complete minimal HTML document. Inside the named #button-chain-app subtree, render exactly one initial button at load and no generated buttons. Give the button user-visible text that distinguishes it from generated buttons without using the reserved completion text finished.

## Acceptance criteria

§ 6.1
A reviewer can identify one #button-chain-app subtree containing exactly one initial button in the initial DOM. No generated-button elements or completion message are present before any click.

## Handoff

§ 7.1
W02 can rely on a completion-message element being absent initially and W03 can rely on one initial button as the starting current last button.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
