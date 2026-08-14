# Step: 01-step-markup

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: #button-chain
- Subscope: `N/A`

## Objective

§ 4.1
Create the document skeleton with one initial button and a stable container for generated buttons.

## Instructions

§ 5.1
Create button-chain.html with valid HTML document structure, a body-level #button-chain root for the button chain, and exactly one initial button visible when the page first loads.

§ 5.2
Do not add generated buttons in the initial markup. Generated buttons must be created only by the script behavior in W03.

## Acceptance criteria

§ 6.1
Source review shows exactly one initial button in the starting markup and a stable #button-chain root where generated buttons can be appended below the current last button.

## Handoff

§ 7.1
W02 can style the completion state and W03 can attach behavior to the initial button without changing the initial markup contract.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
