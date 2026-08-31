# Step: 04-step-test-history

## Ownership

- Goal: `06-pages-history`
- Work unit: `W35`
- Type: `test`

## Change target

- File: `src/plan-overview/tests/pages_history.rs`
- Primary symbol or file scope: `history_pages_render`
- Subscope: `N/A`

## Objective

§ 4.1
Pin that reasons travel with their entries, including when a reason is absent.

## Instructions

§ 5.1
Assert a superseded finding renders its replacement and reason, a rejected alternative renders its rationale, and a corrected paragraph renders both wordings. Fault-inject a discard with no recorded reason and require it to render as flagged rather than be dropped.

## Acceptance criteria

§ 6.1
The test fails if an incomplete entry is omitted from the rendered output, which is the tidy-looking failure mode being guarded.

## Handoff

§ 7.1
W36 confirms the same in the browser on a real plan with real supersessions.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
