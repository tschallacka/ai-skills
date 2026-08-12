# Step: 01-step-initial-markup

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain-root`
- Subscope: `N/A`

## Objective

§ 4.1
Create the initial document structure with one button inside a stable root/container for the chain.

## Instructions

§ 5.1
Create button-chain.html with a minimal HTML skeleton, a visible root element identified as #button-chain-root, and exactly one initial button inside that root. Do not add extra buttons, hidden terminal markup, external dependencies, or implementation styling beyond what W03 owns.

## Acceptance criteria

§ 6.1
button-chain.html opens with exactly one visible button in #button-chain-root and no pre-rendered generated buttons or finished text.

## Handoff

§ 7.1
W02 can attach behavior to the initial button/root without changing the markup target except for event attributes or script references needed by appendNextButton(event).

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
