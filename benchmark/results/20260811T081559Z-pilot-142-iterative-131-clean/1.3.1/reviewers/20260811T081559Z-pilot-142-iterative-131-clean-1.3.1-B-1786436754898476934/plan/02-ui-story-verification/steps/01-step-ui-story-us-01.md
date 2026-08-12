# Step: 01-step-ui-story-us-01

## Ownership

- Goal: `02-ui-story-verification`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `US-01 browser click flow`
- Subscope: `N/A`

## Objective

§ 4.1
Verify US-01 with direct browser clicks after the future implementation exists.

## Instructions

§ 5.1
Open the implemented button-chain.html as a local file in a browser. Follow ui-story-runs/US-01.md exactly: click the current last button five times total, starting with the initial button and ending with generated button 4. Record the URL, decisive screenshot or observation, elapsed waits, and pass or bug status in the story artifacts.

## Acceptance criteria

§ 6.1
US-01 is passed only when direct clicks show exactly one new button after each pre-completion click and the final click clears the document to exactly finished with a visible white border. Any deviation is entered in bugs.md before implementation fixes are planned.

## Handoff

§ 7.1
The initiative can be marked complete only after US-01 evidence is recorded as passed and bugs.md contains no unresolved entries.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
