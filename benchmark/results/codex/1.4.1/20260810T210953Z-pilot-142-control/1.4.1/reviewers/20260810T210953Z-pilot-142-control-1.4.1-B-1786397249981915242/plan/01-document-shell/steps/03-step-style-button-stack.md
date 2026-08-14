# Step: 03-step-style-button-stack

## Ownership

- Goal: `01-document-shell`
- Work unit: `W06`
- Type: `style`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `.chain-button`
- Subscope: `N/A`

## Objective

§ 4.1
Style chain buttons so each generated button appears below the previous current-last button.

## Instructions

§ 5.1
Add a `.chain-button` style target so buttons render as a vertical stack and each generated button appears below the prior current-last button. Do not change completion styling or click behavior in this step.

## Acceptance criteria

§ 6.1
Source inspection shows `.chain-button` is applied to chain buttons and provides block or equivalent vertical layout. Later browser evidence can visually confirm each generated button appears below the previous last button.

## Handoff

§ 7.1
`W03`, `W04`, and `W05` can rely on generated buttons rendering below the prior current-last button when appended to the root.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
