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
Create the future root HTML button-chain subtree with one initial button and the required white-border completion output boundary.

## Instructions

§ 5.1
Future execution only: create button-chain.html with a root #button-chain subtree containing exactly one initially visible button. Keep the completion presentation boundary available for W02 without adding unrelated controls or dependencies. Do not execute this instruction during the current planning proof.

## Acceptance criteria

§ 6.1
The future file target is exactly button-chain.html, the named subtree is #button-chain, and the initial rendered state contains one visible button; this proof confirms the contract text only and creates no file.

## Handoff

§ 7.1
W02 may rely on one initial button inside #button-chain and must attach the current-last-button behavior without changing the markup target.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
