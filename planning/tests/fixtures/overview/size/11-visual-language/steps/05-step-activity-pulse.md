# Step: 05-step-activity-pulse

## Ownership

- Goal: `11-visual-language`
- Work unit: `W68`
- Type: `source`

## Change target

- File: `src/plan-overview/assets/ambient.js`
- Primary symbol or file scope: `activityPulse`
- Subscope: `N/A`

## Objective

§ 4.1
Show that the page is live, readable at a glance from across a room.

## Instructions

§ 5.1
Indicate arrival of real state with a pulse tied to actual events, quiescent when nothing is arriving. It must never animate to imply activity that is not occurring, and must carry a text or shape state as well as motion so it survives the minimal tier.

## Acceptance criteria

§ 6.1
The indicator is quiescent with no incoming state, pulses on real arrival, is readable from a distance, and conveys its state without motion at the minimal tier. US-75 and US-77 apply.

## Handoff

§ 7.1
W73 keeps this legible at every tier; W60 supplies the arrivals.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
