# Step: 01-step-document-structure

## Ownership

- Goal: `01-button-chain-html`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain-root`
- Subscope: `N/A`

## Objective

§ 4.1
Create the HTML document structure containing one initial button and a vertical host for subsequently generated buttons.

## Instructions

§ 5.1
Create button-chain.html as a complete HTML document. Add a root subtree with id button-chain-root containing exactly one visible initial button and a vertical container for generated buttons.

§ 5.2
Use stable, accessible button text or labels so the future browser verification can identify the initial button and each generated current-last button without relying on coordinates.

## Acceptance criteria

§ 6.1
Opening the file before any interaction shows exactly one button and no completion message.

§ 6.2
The document has no external asset, script, stylesheet, or network dependency.

## Handoff

§ 7.1
W02 can rely on #button-chain-root and the initial button existing as the only starting control.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
