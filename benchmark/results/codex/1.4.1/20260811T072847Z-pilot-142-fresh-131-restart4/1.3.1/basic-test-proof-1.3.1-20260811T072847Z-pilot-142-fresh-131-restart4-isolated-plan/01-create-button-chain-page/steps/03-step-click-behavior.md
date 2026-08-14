# Step: 03-step-click-behavior

## Ownership

- Goal: `01-create-button-chain-page`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `button click handler`
- Subscope: `N/A`

## Objective

§ 4.1
Add JavaScript that appends exactly one button below the current last button on each last-button click and replaces the document with the completion state when the fourth generated button is pressed.

## Instructions

§ 5.1
Add JavaScript that tracks how many generated buttons have been appended after the initial button.

§ 5.2
Attach click handling so only the current last button can append the next button; clicks on earlier buttons must not append anything.

§ 5.3
For generated buttons one through three, append exactly one new button below the current last button on each valid click.

§ 5.4
When the fourth generated button is clicked, clear the document content and render the exact lowercase text finished inside the styled completion element.

## Acceptance criteria

§ 6.1
Starting from the initial button, four valid append clicks create four generated buttons, one per click and below the prior last button.

§ 6.2
Clicking the fourth generated button clears previous document content and displays exactly finished with the W02 visible white border.

§ 6.3
No automated or browser verification is run or modified in this step.

## Handoff

§ 7.1
W04 can inspect and simulate the completed file behavior.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
