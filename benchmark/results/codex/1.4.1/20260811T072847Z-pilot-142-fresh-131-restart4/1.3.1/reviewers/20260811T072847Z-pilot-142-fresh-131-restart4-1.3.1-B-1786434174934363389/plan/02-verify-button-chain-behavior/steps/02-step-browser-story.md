# Step: 02-step-browser-story

## Ownership

- Goal: `02-verify-button-chain-behavior`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `browser story US-01 direct button chain flow`
- Subscope: `N/A`

## Objective

§ 4.1
Open the future button-chain.html file in a browser and use direct clicks through the rendered UI to confirm the complete user-visible flow.

## Instructions

§ 5.1
Open the future button-chain.html in a browser using the file URL or approved local route.

§ 5.2
Follow ui-story-runs/US-01.md exactly with direct mouse clicks or equivalent user-facing input; do not use console evaluation, injected events, storage edits, or direct DOM calls.

§ 5.3
Update ui-user-stories.md and the run cache with actual evidence. If the story fails, record a bug in bugs.md and add investigation/fix goals before retesting.

## Acceptance criteria

§ 6.1
US-01 is marked passed with browser evidence showing the exact click flow and the finished completion state.

§ 6.2
bugs.md contains no open bug for US-01.

## Handoff

§ 7.1
The initiative can be marked complete only after this story passes and validation --complete succeeds.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
