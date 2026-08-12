# Step: 01-step-initial-markup

## Ownership

- Goal: `01-create-button-chain-page`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain-root`
- Subscope: `N/A`

## Objective

§ 4.1
Create the single-page HTML body with one initial button inside a stable root container and no extra initial buttons.

## Instructions

§ 5.1
Create button-chain.html as a standalone HTML document with a body container identified as #button-chain-root.

§ 5.2
Place exactly one initial button in that root. Do not include generated buttons in the initial markup.

§ 5.3
Use stable button text that supports the later click sequence, such as Button 1 for the initial button.

## Acceptance criteria

§ 6.1
button-chain.html exists and contains exactly one initial button in #button-chain-root before any script interaction.

§ 6.2
No style or JavaScript behavior is added by this step beyond markup needed for W01.

## Handoff

§ 7.1
W02 can rely on #button-chain-root and the initial button structure being present.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
