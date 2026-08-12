# Step: 03-step-completion-branch

## Ownership

- Goal: `01-button-chain`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `completeChain()`
- Subscope: `fourth-generated-button branch`

## Objective

§ 4.1
Add the completion branch for the fourth generated button that clears the document and renders exactly finished.

## Instructions

§ 5.1
In future execution, add completeChain() and call it only from the branch reached by pressing the fourth generated button. The initial button must not count as generated. The first, second, and third generated button presses must continue to produce the next button instead of completing.

§ 5.2
completeChain() must clear the current document content and render the exact lowercase text finished as the only completion message content.

## Acceptance criteria

§ 6.1
After the fourth generated button is pressed, prior buttons and containers used for the chain are removed or replaced so the document no longer shows the button chain.

§ 6.2
The rendered completion text is exactly finished, lowercase, with no extra punctuation or casing changes.

## Handoff

§ 7.1
W04 can rely on a completion element or equivalent document state containing exact finished text to receive the visible white border style.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
