# Step: 01-step-initial-markup

## Ownership

- Goal: `01-create-button-chain`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: #button-chain-root
- Subscope: `N/A`

## Objective

§ 4.1
Create the minimal HTML document body with exactly one initial button and no pre-rendered generated buttons.

## Instructions

§ 5.1
Create button-chain.html with a document body containing a named root subtree #button-chain-root and exactly one visible initial button. Do not add generated buttons in the static markup.

## Acceptance criteria

§ 6.1
Pass when the future file has exactly one initial button at load, the root subtree is identifiable as #button-chain-root, and no generated buttons are pre-rendered.

## Handoff

§ 7.1
W02 can rely on a single initial button target inside #button-chain-root.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
