# Step: 02-step-browser-story

## Ownership

- Goal: `02-prove-button-chain-behavior`
- Work unit: `W06`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `Browser flow: click initial, generated one, generated two, generated three, generated four`
- Subscope: `N/A`

## Objective

§ 4.1
Run the future browser story through real clicks and confirm the document clears to finished with a visible white border.

## Instructions

§ 5.1
In future execution, open button-chain.html in a browser and perform US-01 exactly from ui-story-runs/US-01.md using normal mouse clicks only.

§ 5.2
Record the actual URL or file path, observed button counts after each click, final screenshot or visual evidence, story status, and any bug-register entries if the story fails.

## Acceptance criteria

§ 6.1
US-01 is passed only when each of the first four clicks appends exactly one button below the previous last button and the fifth click leaves only finished with a visible white border.

## Handoff

§ 7.1
The initiative can be accepted when W06 passes, ui-user-stories.md records passed evidence, and bugs.md has no open rows.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
