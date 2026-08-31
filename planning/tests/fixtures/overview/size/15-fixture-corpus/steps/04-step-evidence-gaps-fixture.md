# Step: 04-step-evidence-gaps-fixture

## Ownership

- Goal: `15-fixture-corpus`
- Work unit: `W94`
- Type: `data`

## Change target

- File: `planning/tests/fixtures/overview/evidence-gaps`
- Primary symbol or file scope: `evidence gaps fixture`
- Subscope: `N/A`

## Objective

§ 4.1
A fixture carrying the evidence edge cases: a finding with a blank work-unit cell, a finding with a gated fix key, a coverage outcome with no proving unit, and a review cycle that recorded no findings.

## Instructions

§ 5.1
Build planning/tests/fixtures/overview/evidence-gaps with the plan helpers, then record four evidence-side gaps: a finding with a blank work-unit cell, a finding whose fix is behind a gated key, a coverage outcome naming no proving unit, and a review cycle that recorded no findings. Where a helper refuses to write a gap, record that refusal in the step rather than bypassing the helper: a state the helpers cannot produce is not a state the page must render.

## Acceptance criteria

§ 6.1
Each of the four gaps is present, or is recorded as unreachable with the refusing helper named. The fixture validates except where the gap is the point, and the step says which validation result is expected.

## Handoff

§ 7.1
The findings and coverage stories have a fixture whose gaps are the thing under observation, so a page that hides a gap fails instead of having nothing to hide.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
