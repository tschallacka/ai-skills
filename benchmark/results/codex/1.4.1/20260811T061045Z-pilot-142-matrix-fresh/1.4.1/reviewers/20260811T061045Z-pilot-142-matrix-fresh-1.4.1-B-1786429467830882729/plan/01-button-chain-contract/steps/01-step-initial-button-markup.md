# Step: 01-step-initial-button-markup

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
Create the document body with exactly one initial button and a root container for the button chain.

## Instructions

§ 5.1
In the future implementation, create button-chain.html with valid HTML structure and a visible root element identified as #button-chain-root.

§ 5.2
Place exactly one initial button in the root. Do not add pre-rendered generated buttons or the finished message in the initial state.

## Acceptance criteria

§ 6.1
Opening the future file initially shows one and only one button in the chain root.

§ 6.2
The initial markup gives later script and style targets stable elements to operate on without requiring any second file.

## Handoff

§ 7.1
W02 can attach click behavior to the root or its button because W01 provides the single initial interactive element.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
