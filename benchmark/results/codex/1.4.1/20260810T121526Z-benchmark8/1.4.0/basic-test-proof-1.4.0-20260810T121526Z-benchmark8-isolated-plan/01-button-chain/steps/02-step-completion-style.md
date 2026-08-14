# Step: 02-step-completion-style

## Ownership

- Goal: `01-button-chain`
- Work unit: `W02`
- Type: `style`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `.completion-message`
- Subscope: `N/A`

## Objective

§ 4.1
Define the visible white border and presentation for the terminal finished message.

## Instructions

§ 5.1
In button-chain.html, define the .completion-message selector with an explicit `1px solid white` border and a non-white background (for example, black) so the future terminal element visibly renders a distinguishable white border while retaining exact text visibility. Keep this style target separate from handler logic.

## Acceptance criteria

§ 6.1
When the terminal element uses .completion-message, a future browser observer can visibly identify the explicit 1px solid white border around the lowercase completion text against its non-white background.

## Handoff

§ 7.1
W03 applies the named .completion-message hook when it renders the terminal state; W04 checks the border visually.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
