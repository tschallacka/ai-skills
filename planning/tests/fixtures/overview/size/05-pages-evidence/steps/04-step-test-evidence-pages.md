# Step: 04-step-test-evidence-pages

## Ownership

- Goal: `05-pages-evidence`
- Work unit: `W30`
- Type: `test`

## Change target

- File: `src/plan-overview/tests/pages_evidence.rs`
- Primary symbol or file scope: `evidence_pages_render`
- Subscope: `N/A`

## Objective

§ 4.1
Pin that a test page carries its procedure and that coverage renders every listed id.

## Instructions

§ 5.1
Assert a test page contains the companion procedure text rather than a reference, and that a coverage row renders each of its unit ids. Fault-inject a companion with no automated-tests section and a coverage row listing eight units.

## Acceptance criteria

§ 6.1
The test fails if a procedure is replaced by a reference or if any id in a row is dropped or abbreviated in the rendered output.

## Handoff

§ 7.1
W31 confirms the same in the browser, where clipping rather than omission is the risk.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
