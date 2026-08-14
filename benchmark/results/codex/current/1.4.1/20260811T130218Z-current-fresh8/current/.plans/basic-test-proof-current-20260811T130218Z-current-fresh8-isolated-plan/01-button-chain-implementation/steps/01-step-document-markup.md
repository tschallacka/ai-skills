# Step: 01-step-document-markup

## Ownership

- Goal: `01-button-chain-implementation`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain-root`
- Subscope: `N/A`

## Objective

§ 4.1
Create a valid standalone HTML document containing exactly one initial button in the body and a container/order that supports generated buttons below it.

## Instructions

§ 5.1
Create button-chain.html as a valid standalone HTML document with a body-level root identified as #button-chain-root.

§ 5.2
Place exactly one initial visible button in the root. Do not add generated buttons, completion text, scripts beyond what later work units own, or external dependencies in this step.

## Acceptance criteria

§ 6.1
button-chain.html exists after future execution and contains one initial visible button before any interaction.

§ 6.2
The root for the button chain is named #button-chain-root and supports appending generated buttons in visual order below the current last button.

## Handoff

§ 7.1
W02 and W03 can rely on a stable #button-chain-root and a single initial button target.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
