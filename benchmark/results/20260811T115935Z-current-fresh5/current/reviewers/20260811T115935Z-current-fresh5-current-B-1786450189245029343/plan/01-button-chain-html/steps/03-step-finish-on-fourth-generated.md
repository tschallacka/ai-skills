# Step: 03-step-finish-on-fourth-generated

## Ownership

- Goal: `01-button-chain-html`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `finishOnFourthGeneratedButton()`
- Subscope: `N/A`

## Objective

§ 4.1
Implement the terminal branch so pressing the fourth generated button clears the document and renders exactly finished.

## Instructions

§ 5.1
Implement finishOnFourthGeneratedButton() in button-chain.html. When the valid current-last click target is generated button 4, clear the document content and render a single completion element.

§ 5.2
Set the completion element text with textContent or equivalent exact text assignment to finished. Do not include extra words, capitalization, punctuation, or hidden duplicate completion text.

## Acceptance criteria

§ 6.1
The fourth generated button click removes the button chain from the visible document.

§ 6.2
The post-completion visible text is exactly finished in lowercase.

§ 6.3
No further button click target remains after completion.

## Handoff

§ 7.1
W04 can rely on a single completion element that can receive the .completion-message styling.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
