# Step: 01-step-verify-append-story

## Ownership

- Goal: `02-user-story-verification`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `Browser flow US-01 incremental append`
- Subscope: `N/A`

## Objective

§ 4.1
Verify by direct browser clicks that each press on the current last button appends exactly one button below it before completion.

## Instructions

§ 5.1
After future implementation and W07 contract review, open button-chain.html in a browser and execute ui-story-runs/US-01.md exactly: click the current last button four times, each time selecting the newly appended button as the next last button.

## Acceptance criteria

§ 6.1
US-01 passes only if there are exactly five visible buttons after four append clicks, representing one initial button plus four generated buttons, each click added exactly one button below the previous last button, and finished is not visible yet.

## Handoff

§ 7.1
W06 starts from the passing US-01 state with five buttons visible and the fourth generated button as the current last button.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
