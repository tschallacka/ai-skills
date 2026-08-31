# Step: 02-step-autoplay-follow

## Ownership

- Goal: `08-autoplay`
- Work unit: `W42`
- Type: `source`

## Change target

- File: `src/plan-overview/assets/autoplay.js`
- Primary symbol or file scope: `autoplayFollow`
- Subscope: `N/A`

## Objective

§ 4.1
Follow the work without a human driving, while never stealing navigation from a reader.

## Instructions

§ 5.1
When autoplay is on, navigate to the active step and keep following as the state changes. When it is off, never move the page, but indicate that newer state exists. Toggling on moves to the active step at that moment; toggling off leaves the reader where autoplay last took them.

## Acceptance criteria

§ 6.1
With autoplay off the page does not navigate across two state changes and shows that newer state exists; with it on the page follows each change; the transition is visible rather than an unexplained jump. US-09, US-26 and US-27 apply.

## Handoff

§ 7.1
W67 supplies the transition so a follow reads as movement rather than a cut.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
