# Step: 01-step-initial-markup

## Ownership

- Goal: `01-button-chain-html`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: #button-chain
- Subscope: `N/A`

## Objective

§ 4.1
Create the initial body structure with exactly one visible initial button and no completion text at load.

## Instructions

§ 5.1
Future executor creates button-chain.html with a minimal valid HTML document and a body subtree containing exactly one button element at load. The button should be identifiable as the initial/current last button and should not be accompanied by finished text or generated-button placeholders.

## Acceptance criteria

§ 6.1
Loading the future file before interaction exposes exactly one visible button and zero elements or text nodes whose rendered content equals finished.

## Handoff

§ 7.1
W02 and W03 can rely on a single initial button existing as the only chain control at load.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
