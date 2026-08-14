# Step: 01-step-run-ui-story

## Ownership

- Goal: `02-verify-button-chain-flow`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `Browser UI story US-01`
- Subscope: `N/A`

## Objective

§ 4.1
Open the implemented button-chain.html in a browser and use direct clicks to verify one-button appends, fourth-generated terminal clearing, exact finished text, and visible white border.

## Instructions

§ 5.1
Open the implemented button-chain.html in a browser after future implementation. Use the cached US-01 sequence: confirm the initial state, click the initial button, generated button 1, generated button 2, generated button 3, and generated button 4 as five separate direct interactions. Record direct-click evidence and intermediate button counts after each click.

§ 5.2
Do not use console commands, JavaScript evaluation, DOM mutation, storage edits, direct API calls, or injected events to trigger the behavior.

## Acceptance criteria

§ 6.1
US-01 passes only when each append click adds exactly one button below the previous last button and the terminal click leaves only exact lowercase text finished with a visible white border.

§ 6.2
Any mismatch is recorded in bugs.md with reproduction, severity, investigation goal, fix goal, retest story, and open status before implementation resumes.

## Handoff

§ 7.1
When passed, mark US-01 passed with evidence and complete the plan. When failed, follow the bug feedback loop before acceptance.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
