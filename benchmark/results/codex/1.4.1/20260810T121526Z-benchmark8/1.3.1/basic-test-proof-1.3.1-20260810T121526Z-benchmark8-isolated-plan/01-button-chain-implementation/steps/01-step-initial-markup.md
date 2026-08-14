# Step: 01-step-initial-markup

## Ownership

- Goal: `01-button-chain-implementation`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain`
- Subscope: `N/A`

## Objective

§ 4.1
Define the document shell and one initial button as the only initial interactive control.

## Instructions

§ 5.1
Place a document shell and exactly one initial button in the body subtree; give the control a stable accessible label for later mouse-based validation.

## Acceptance criteria

§ 6.1
The future file opens with exactly one visible button and no generated buttons or completion message.

## Handoff

§ 7.1
W02 can bind the initial button as the current last button without needing an additional initial control.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
