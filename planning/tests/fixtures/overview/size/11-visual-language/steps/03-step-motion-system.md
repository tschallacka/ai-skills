# Step: 03-step-motion-system

## Ownership

- Goal: `11-visual-language`
- Work unit: `W66`
- Type: `style`

## Change target

- File: `src/plan-overview/assets/motion.css`
- Primary symbol or file scope: .motion
- Subscope: `N/A`

## Objective

§ 4.1
Declare motion once so a transition cannot be hand-tuned per element.

## Instructions

§ 5.1
Define durations and easings as tokens, each with a stated purpose. Provide one reduced-motion block that neutralises duration and transform while leaving every state change applied. Motion tokens are the only source of timing for transitions and animations.

## Acceptance criteria

§ 6.1
No animation or transition declares its own duration or easing outside the tokens, and under the reduced-motion preference every state change still occurs with no motion. US-69, US-78 and US-83 apply.

## Handoff

§ 7.1
W67 and W56 consume these; W71 declares which of them each tier disables.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
