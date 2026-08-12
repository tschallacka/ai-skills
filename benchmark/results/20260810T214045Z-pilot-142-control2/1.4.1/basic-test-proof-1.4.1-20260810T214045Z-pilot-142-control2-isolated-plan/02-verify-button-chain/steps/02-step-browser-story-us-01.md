# Step: 02-step-browser-story-us-01

## Ownership

- Goal: `02-verify-button-chain`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `US-01 browser story`
- Subscope: `N/A`

## Objective

§ 4.1
Run the direct browser user story that clicks the current last button through the fourth generated button and observes the finished state.

## Instructions

§ 5.1
Run ui-story-runs/US-01.md after implementation. Use only normal browser input: open the local file, click the visible current last button, then click each newly appended bottom-most button until the fourth generated button is clicked. Do not use console commands, JavaScript evaluation, direct DOM mutation, storage edits, or injected events.

## Acceptance criteria

§ 6.1
US-01 passes only when the first three accepted generated-button clicks each add exactly one button below the previous last button and the fourth generated-button click leaves the page showing only exact lowercase finished with a visible white border.

## Handoff

§ 7.1
W06 can audit final artifacts after US-01 records passed evidence and bugs.md has no open bug rows.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
