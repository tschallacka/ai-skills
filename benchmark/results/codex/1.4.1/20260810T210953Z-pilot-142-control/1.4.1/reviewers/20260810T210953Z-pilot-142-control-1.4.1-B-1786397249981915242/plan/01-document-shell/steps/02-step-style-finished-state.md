# Step: 02-step-style-finished-state

## Ownership

- Goal: `01-document-shell`
- Work unit: `W02`
- Type: `style`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `.completion-state`
- Subscope: `N/A`

## Objective

§ 4.1
Add visible completion styling with a white border around the finished state.

## Instructions

§ 5.1
Add styling for `.completion-state` in `button-chain.html` so the terminal message has a visible white border. Do not alter click behavior or add generated buttons.

## Acceptance criteria

§ 6.1
Source inspection shows a `.completion-state` rule or equivalent scoped style that makes the finished state visibly bordered in white.

## Handoff

§ 7.1
`W03` can render the terminal completion node using `.completion-state` and rely on the style to provide the visible white border.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
