# Step: 01-step-tokens

## Ownership

- Goal: `11-visual-language`
- Work unit: `W64`
- Type: `style`

## Change target

- File: `src/plan-overview/assets/tokens.css`
- Primary symbol or file scope: .tokens
- Subscope: `N/A`

## Objective

§ 4.1
Put every colour, luminance step, spacing value and type size in one place so none can be introduced ad hoc.

## Instructions

§ 5.1
Define the palette, luminance steps, spacing scale and type scale as custom properties on the root. Neutrals carry a slight hue bias toward the accent rather than being pure grey. Define the complete light palette on the bare root, redefine only the tokens under the dark preference guarded so an explicit choice wins, and again under an explicit dark attribute.

## Acceptance criteria

§ 6.1
No page or component declares a colour outside the tokens, both themes resolve completely in all three preference states, and the body paints an explicit token background rather than inheriting a transparent ground.

## Handoff

§ 7.1
W65 and W66 build on these; W70 measures contrast against the composited result rather than the token.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
