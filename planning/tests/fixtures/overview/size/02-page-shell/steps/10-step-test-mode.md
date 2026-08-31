# Step: 10-step-test-mode

## Ownership

- Goal: `02-page-shell`
- Work unit: `W52`
- Type: `test`

## Change target

- File: `src/plan-overview/tests/mode.rs`
- Primary symbol or file scope: `mode_derivation`
- Subscope: `N/A`

## Objective

§ 4.1
Test lifecycle derivation including the exact approved-with-zero-steps empty-approved fixture as ambiguous with the contradiction named while its page remains readable.

## Instructions

§ 5.1
Assert pending review with no started steps is planning; approved with started steps is implementing; all steps and verification complete is complete; and the approved-with-zero-steps fixture is ambiguous because approval contradicts the empty execution shape. The same fixture must still render a readable empty plan. Fault-inject by approving a plan with no steps and require the ambiguous mode and readable empty rendering rather than a guessed mode or an error.

## Acceptance criteria

§ 6.1
Every lifecycle case is pinned. The approved-with-zero-steps fixture produces ambiguous with the contradiction named and renders empty content without an error. The fault injection fails if derivation guesses planning or implementing or if the page suppresses the empty plan.

## Handoff

§ 7.1
W52 is the guard for W50; W53 and W51 assume its outcomes.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
