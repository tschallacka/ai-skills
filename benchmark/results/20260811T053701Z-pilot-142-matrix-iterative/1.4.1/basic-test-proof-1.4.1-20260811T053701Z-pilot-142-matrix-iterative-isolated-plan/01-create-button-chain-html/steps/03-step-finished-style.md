# Step: 03-step-finished-style

## Ownership

- Goal: `01-create-button-chain-html`
- Work unit: `W03`
- Type: `style`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `.completion-message`
- Subscope: `N/A`

## Objective

§ 4.1
Add the completion-state styling rule that makes the lowercase finished text visibly bordered in white after document clearing.

## Instructions

§ 5.1
Add a .completion-message CSS rule in button-chain.html that makes the terminal finished text visibly bordered in white. Keep the style local to the single HTML file.

§ 5.2
Ensure the border remains visible against the terminal state's background; if necessary, set a contrasting background on the body or completion element without introducing decorative assets.

## Acceptance criteria

§ 6.1
The terminal finished text has a CSS border whose color is white and whose width/style make it visibly present.

§ 6.2
The styling does not hide, alter case, or surround additional text beyond finished.

## Handoff

§ 7.1
W04 may inspect button-chain.html for the .completion-message rule and exact finished text contract.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
