# Step: 01-step-browser-story-us-01

## Ownership

- Goal: `02-validate-button-chain`
- Work unit: `W06`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `Browser UI story US-01`
- Subscope: `N/A`

## Objective

§ 4.1
Open the implemented button-chain.html in a browser and click the current last button until the fourth generated button clears the document and shows finished with a visible white border.

## Instructions

§ 5.1
Open the implemented local button-chain.html in a browser. Use only direct mouse clicks on the visible current last button: initial button, generated button 1, generated button 2, generated button 3, generated button 4.

§ 5.2
After each click before completion, confirm exactly one new button appears below the previous last button. After clicking generated button 4, confirm the prior document content is cleared and the page shows exactly finished with a visible white border.

## Acceptance criteria

§ 6.1
US-01 is marked passed only when the browser evidence shows the exact click sequence, exactly-one append after each eligible click, no stale-button append path, clearing on generated button 4, and bordered lowercase finished text.

§ 6.2
If any condition fails, mark US-01 bug found, add a bug row with reproduction and evidence, and create investigation/fix work units before retesting.

## Handoff

§ 7.1
Completion handoff is the updated ui-story-runs/US-01.md evidence, updated ui-user-stories.md status, clear bugs.md state, and completed progress trackers.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
