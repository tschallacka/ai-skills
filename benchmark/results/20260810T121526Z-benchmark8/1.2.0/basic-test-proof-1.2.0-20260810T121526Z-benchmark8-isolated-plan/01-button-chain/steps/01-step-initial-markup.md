# Step 01: initial markup

## Ownership
- Goal: `01-button-chain`
- Work unit: `W01`
- Type: `markup`

## Change target
- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain`
- Subscope: `N/A`

## Objective
Create the stable document subtree with exactly one initial button.

## Instructions
1. Create the future standalone document and add exactly one accessible button named `Button 0` inside `#button-chain`; do not add generated buttons to the initial markup.

## Acceptance criteria
- The initial render contains one visible button named `Button 0` and the named container.

## Handoff
- W02 can bind the append behavior to the one initial current-last button.

## Atomicity check
- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
