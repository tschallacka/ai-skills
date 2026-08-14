# Step: 01-step-initial-markup

## Ownership

- Goal: `01-button-chain-behavior`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain-root`
- Subscope: `N/A`

## Objective

§ 4.1
Create the document body region with exactly one initial button and no generated buttons at load.

## Instructions

§ 5.1
Create `button-chain.html` as a valid standalone HTML document. In the body, add a single root region with id `button-chain-root` and exactly one button inside it at initial load. Do not add generated buttons in static markup.

## Acceptance criteria

§ 6.1
Opening the future file before any interaction shows one clickable button and no `finished` text. The root region is the single place later behavior appends generated buttons.

## Handoff

§ 7.1
W02 can rely on a stable root region and one initial button being present at load.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
