# Step: 01-step-markup

## Ownership

- Goal: `01-button-chain-contract`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain`
- Subscope: `N/A`

## Objective

§ 4.1
Create the future root HTML button-chain subtree with one initial button and a completion output boundary capable of showing a white border.

## Instructions

§ 5.1
During future execution only, create repository-root button-chain.html with a valid standalone document. Define #button-chain as the sole initial user-facing subtree, place exactly one visible initial button in it, and establish block or flex-column layout so every later button occupies a new row below its predecessor. Include the completion presentation boundary needed to render exact text finished inside a visible white border, without implementing appendButtonChain() in this unit.

## Acceptance criteria

§ 6.1
A source review of only #button-chain shows one and only one initial button, zero generated buttons, an explicit vertical stacking rule, no unrelated UI, and a defined completion presentation whose rendered border is white and can be visually distinguished. No executable append behavior is added by W01.

## Handoff

§ 7.1
W02 may rely on #button-chain, the initial button, the vertical insertion boundary, and the completion presentation contract; W02 alone owns executable behavior.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
