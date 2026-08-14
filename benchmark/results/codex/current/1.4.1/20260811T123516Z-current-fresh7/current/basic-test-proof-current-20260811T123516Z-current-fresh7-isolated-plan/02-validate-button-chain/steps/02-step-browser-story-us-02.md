# Step: 02-step-browser-story-us-02

## Ownership

- Goal: `02-validate-button-chain`
- Work unit: `W08`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `Browser UI story US-02`
- Subscope: `N/A`

## Objective

§ 4.1
After at least one generated button exists, click an older non-last button and verify it does not append another button before continuing the chain.

## Instructions

§ 5.1
Open the implemented local button-chain.html in a browser. Click the initial button to create generated button 1, then click the original initial button again while it is no longer the last button.

§ 5.2
Confirm no new button appears after the stale click and that generated button 1 remains the current last button for continuing the valid chain.

## Acceptance criteria

§ 6.1
US-02 is marked passed only when direct browser evidence shows the stale initial-button click does not append generated button 2 or any other button.

§ 6.2
If the stale click appends anything, mark US-02 bug found and add investigation/fix work before retesting.

## Handoff

§ 7.1
Handoff is updated ui-story-runs/US-02.md evidence, updated ui-user-stories.md status, and a bugs.md state with no unresolved stale-click bug.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
