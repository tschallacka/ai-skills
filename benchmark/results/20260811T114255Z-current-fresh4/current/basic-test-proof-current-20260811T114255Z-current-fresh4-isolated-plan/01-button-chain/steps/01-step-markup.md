# Step: 01-step-markup

## Ownership

- Goal: `01-button-chain`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain-root`
- Subscope: `N/A`

## Objective

§ 4.1
Create the document body structure with one initial button and a stable container for generated buttons.

## Instructions

§ 5.1
In future execution, create button-chain.html with a complete HTML document, a body-level container identified as #button-chain-root, and exactly one initial visible button inside it. Give the button a stable accessible label such as Add button and structure the container so generated buttons can be appended below the current last button.

§ 5.2
Do not add append logic, completion logic, or completion styling in this step; those belong to W02, W03, and W04.

## Acceptance criteria

§ 6.1
button-chain.html contains one initial button before interaction and no generated buttons are present in the initial markup.

§ 6.2
The DOM exposes a clear #button-chain-root subtree that W02 can use for append operations.

## Handoff

§ 7.1
W02 can rely on a stable #button-chain-root container and a single initial last button to attach click handling.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
