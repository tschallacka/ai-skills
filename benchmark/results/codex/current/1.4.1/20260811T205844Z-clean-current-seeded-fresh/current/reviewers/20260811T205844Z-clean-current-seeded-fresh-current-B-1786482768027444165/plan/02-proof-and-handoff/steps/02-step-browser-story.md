# Step: 02-step-browser-story

## Ownership

- Goal: `02-proof-and-handoff`
- Work unit: `W06`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `Browser story US-01`
- Subscope: `N/A`

## Objective

§ 4.1
Run the planned user-facing browser story by clicking the current last button until the fourth generated button is pressed and record pass/fail evidence.

## Instructions

§ 5.1
Future executor runs US-01 with normal browser input only. Load the local file, click the bottom-most visible button five times from the initial state, and record the URL/file path, visible button count progression, final screenshot or observation, exact text, and visible white border evidence.

## Acceptance criteria

§ 6.1
US-01 passes only when the browser evidence shows one initial button, exactly one appended button after each of the first four clicks, and after the fifth click only finished remains with a visible white border. It remains untested during this planning-only proof.

## Handoff

§ 7.1
Completion handoff is the updated ui-story-runs/US-01.md evidence and no open bugs in bugs.md.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
