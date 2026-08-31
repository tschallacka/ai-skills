# Step: 03-step-autoplay-tabs

## Ownership

- Goal: `08-autoplay`
- Work unit: `W43`
- Type: `source`

## Change target

- File: `src/plan-overview/assets/autoplay.js`
- Primary symbol or file scope: `autoplayTabs`
- Subscope: `N/A`

## Objective

§ 4.1
Make concurrent work legible instead of picking one arbitrary current step.

## Instructions

§ 5.1
Render one tab per active state, let the reader select one, and follow the selected tab. A tab vanishes as soon as its state stops being active. When the last tab vanishes, state that there is no active step rather than holding the last one.

## Acceptance criteria

§ 6.1
Two active states produce two tabs, selection decides what is followed, ending one state removes its tab, and ending all of them produces an explicit no active step. US-10 and US-28 apply.

## Handoff

§ 7.1
W61 preserves the selected tab across a live update.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
