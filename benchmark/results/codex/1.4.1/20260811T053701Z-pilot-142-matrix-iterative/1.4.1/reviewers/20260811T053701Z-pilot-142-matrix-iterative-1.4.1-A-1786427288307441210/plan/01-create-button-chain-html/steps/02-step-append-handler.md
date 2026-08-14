# Step: 02-step-append-handler

## Ownership

- Goal: `01-create-button-chain-html`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `button-chain script`
- Subscope: `click handler`

## Objective

§ 4.1
Add JavaScript so only the current last button appends exactly one new button below itself, with generated-button counting tracked deterministically.

## Instructions

§ 5.1
Add script behavior in button-chain.html for the button-chain click handler. Track generated buttons separately from the initial button so generated button 4 is the terminal trigger when clicked.

§ 5.2
On click, first confirm the clicked target is the current last button. If it is not the last button, leave the document unchanged. If it is the initial button or generated buttons 1 through 3, append exactly one new generated button below the current last button.

§ 5.3
When the current last button is generated button 4 and it is clicked, clear the document body and render only the completion message text finished. Do not use external libraries or additional files.

## Acceptance criteria

§ 6.1
Clicking the initial button, generated button 1, generated button 2, and generated button 3 appends exactly one new button below the current last button per click.

§ 6.2
Clicking any earlier non-last button does not append a button.

§ 6.3
Clicking generated button 4 clears prior content and renders exact lowercase text finished.

## Handoff

§ 7.1
W03 may rely on the terminal state rendering an element or body state that can receive the .completion-message border styling.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
