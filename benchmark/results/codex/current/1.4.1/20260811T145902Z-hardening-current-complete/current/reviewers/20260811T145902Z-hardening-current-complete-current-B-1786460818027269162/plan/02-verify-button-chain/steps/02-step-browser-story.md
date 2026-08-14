# Step: 02-step-browser-story

## Ownership

- Goal: `02-verify-button-chain`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `US-01 browser flow`
- Subscope: `N/A`

## Objective

§ 4.1
Run the browser user story by clicking the current last button until the fourth generated button triggers the finished state.

## Instructions

§ 5.1
After future implementation and W04 success, open button-chain.html in a browser and use direct clicks on the current visible last button. Click the initial button to append generated button 1, then click generated buttons 1, 2, 3, and 4 as they become last.

§ 5.2
Record the visible result after each click and stop when the document is cleared and the exact finished message with a visible white border is shown.

## Acceptance criteria

§ 6.1
US-01 passes only with browser evidence showing direct clicks, exactly one appended button per current-last click, document clearing on the fourth generated button, exact text finished, and a visible white border.

§ 6.2
Any mismatch becomes a bug entry with investigation, fix, and retest work units before the initiative can complete.

## Handoff

§ 7.1
This step produces final user-story evidence for handoff.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
