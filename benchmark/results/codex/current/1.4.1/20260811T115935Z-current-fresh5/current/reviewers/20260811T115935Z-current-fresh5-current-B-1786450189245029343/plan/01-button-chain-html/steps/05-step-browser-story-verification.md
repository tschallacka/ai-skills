# Step: 05-step-browser-story-verification

## Ownership

- Goal: `01-button-chain-html`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `Browser user story US-01`
- Subscope: `N/A`

## Objective

§ 4.1
Verify through real browser clicks that the button chain appends exactly one button per click and completes with bordered finished text after the fourth generated button is pressed.

## Instructions

§ 5.1
After W01-W04 are implemented in a future execution session, open button-chain.html in a browser and run UI story US-01 from ui-story-runs/US-01.md using real mouse clicks only.

§ 5.2
Record the actual URL or local file path, the observed button count after each click, the terminal visible text, and whether the white border is visible. Do not use console evaluation, DOM mutation, storage edits, or direct script calls as passing evidence.

## Acceptance criteria

§ 6.1
US-01 is marked passed only after the visible UI shows exactly one appended button after each non-terminal current-last click and only bordered finished text after clicking generated button 4.

§ 6.2
If any story result differs, record it in bugs.md and follow the bug feedback loop before completion.

## Handoff

§ 7.1
The initiative can be considered complete only when the future browser story evidence passes and no required UI bug remains open.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
