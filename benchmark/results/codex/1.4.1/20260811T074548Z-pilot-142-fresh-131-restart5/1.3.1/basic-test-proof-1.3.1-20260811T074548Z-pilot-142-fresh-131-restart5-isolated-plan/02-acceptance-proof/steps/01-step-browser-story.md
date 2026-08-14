# Step: 01-step-browser-story

## Ownership

- Goal: `02-acceptance-proof`
- Work unit: `W06`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `US-01 browser click-chain flow`
- Subscope: `N/A`

## Objective

§ 4.1
Run the direct browser story by clicking through the button chain and observing the finished state.

## Instructions

§ 5.1
After implementation, open `button-chain.html` in a browser and execute `US-01` exactly as cached: click the initial current last button, then generated buttons 1, 2, 3, and 4 as each becomes the current last button. Do not use console commands, injected events, direct DOM mutation, storage edits, or direct API calls.

## Acceptance criteria

§ 6.1
The story passes only if each pre-completion click appends exactly one button below the previous last button and the final click clears the buttons and displays only `finished` with a visible white border.

## Handoff

§ 7.1
The initiative can be accepted after W06 passes and `ui-user-stories.md`, `ui-story-runs/US-01.md`, and `bugs.md` show no unresolved UI bug.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
