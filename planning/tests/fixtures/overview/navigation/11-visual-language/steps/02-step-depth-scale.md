# Step: 02-step-depth-scale

## Ownership

- Goal: `11-visual-language`
- Work unit: `W65`
- Type: `style`

## Change target

- File: `src/plan-overview/assets/depth.css`
- Primary symbol or file scope: .panel
- Subscope: `N/A`

## Objective

§ 4.1
Make layering deliberate, since translucent panels are the fastest route to an unreadable page.

## Instructions

§ 5.1
Define a named depth scale with background blur, border luminance and shadow per level, and a stated maximum number of levels. Content contrast is computed against the composited background, not the token colour, because a panel over a bright region is the case that fails.

## Acceptance criteria

§ 6.1
Every panel sits on a named level, no page exceeds the stated maximum, and text over a panel overlaying a bright region still meets contrast. US-79 applies.

## Handoff

§ 7.1
W70 measures the composited contrast; the tier system reduces blur first when stepping down.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
