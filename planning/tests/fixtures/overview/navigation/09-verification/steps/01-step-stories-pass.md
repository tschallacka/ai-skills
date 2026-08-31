# Step: 01-step-stories-pass

## Ownership

- Goal: `09-verification`
- Work unit: `W46`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `browser stories on the navigation fixture`
- Subscope: `N/A`

## Objective

§ 4.1
Confirm every journey passed with real interaction, not with a screenshot.

## Instructions

§ 5.1
Work the story table one story at a time, each on the fixture its own row names: the navigation fixture for the navigation and graph stories, and the anomalies, evidence-gaps, complete, empty-approved, fresh or malformed-state fixture for the stories that name one. Record per story the control used, the action taken and what was observed. An earlier version of this instruction said to work the whole table on the 82-unit fixture; adversarial finding AR-21 recorded that eleven stories name a state that fixture does not contain, so those stories could not have passed on it.

## Acceptance criteria

§ 6.1
Every story is passed, bug found, or explicitly excluded with an approved reason; no story passes on a DOM read, a console command or a screenshot alone; every bug row carries its evidence and its retest.

## Handoff

§ 7.1
The story table plus the bug register is the acceptance record for the whole plan.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
