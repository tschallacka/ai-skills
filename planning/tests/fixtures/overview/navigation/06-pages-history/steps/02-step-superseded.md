# Step: 02-step-superseded

## Ownership

- Goal: `06-pages-history`
- Work unit: `W33`
- Type: `source`

## Change target

- File: `src/plan-overview/src/pages/history.rs`
- Primary symbol or file scope: `render_superseded()`
- Subscope: `N/A`

## Objective

§ 4.1
Explain why a finding stopped being open, not merely that it did.

## Instructions

§ 5.1
Render superseded and resolved findings with what replaced them and the recorded reason, and keep them reachable rather than hiding them once resolved. Where a resolution names a mutation, show it, since that is what distinguishes a verified fix from an assertion.

## Acceptance criteria

§ 6.1
A superseded finding shows its replacement and reason; a resolved finding remains reachable on a completed plan; a resolution with no stated mutation is visible as unproven rather than reading as verified. US-07 and US-18 apply.

## Handoff

§ 7.1
W27 links a finding to this view; the two share one derivation of finding status.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
