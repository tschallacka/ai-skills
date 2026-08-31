# Step: 03-step-unit-page

## Ownership

- Goal: `04-pages-primary`
- Work unit: `W23`
- Type: `source`

## Change target

- File: `src/plan-overview/src/pages/unit.rs`
- Primary symbol or file scope: `render_unit()`
- Subscope: `N/A`

## Objective

§ 4.1
Show one work unit in full, so a reviewer never needs the step file open beside the page.

## Instructions

§ 5.1
Render the change target as its four distinct fields, file, primary symbol, subscope and type, plus objective, instructions, acceptance criteria, handoff and the atomicity check. A verification unit with no file states that rather than showing an empty field, and a subscope of not applicable is shown as such.

## Acceptance criteria

§ 6.1
All four target fields are distinguishable, the atomicity claim is visible with the unit, long instructions wrap without clipping, and a long path does not overflow its container. US-19, US-37 and US-55 apply.

## Handoff

§ 7.1
W24 adds the edge block; W27 links findings here; W28 links the unit a test proves.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
