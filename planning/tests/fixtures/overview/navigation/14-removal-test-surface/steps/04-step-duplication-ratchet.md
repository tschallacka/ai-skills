# Step: 04-step-duplication-ratchet

## Ownership

- Goal: `14-removal-test-surface`
- Work unit: `W88`
- Type: `test`

## Change target

- File: `planning/tests/test-duplication-ratchet.sh`
- Primary symbol or file scope: `canonicalisation site count`
- Subscope: `N/A`

## Objective

§ 4.1
Correct the ratchet entry that counts render-plan-overview.sh cells() as a canonicalisation site. The count is the assertion, so removing a site without correcting it reddens the suite.

## Instructions

§ 5.1
In planning/tests/test-duplication-ratchet.sh, correct the canonicalisation-site count so it no longer counts the cells function in the deleted renderer. Lower the number by exactly the sites removed; do not relax the comparison or widen the tolerance to make it pass, which would retire the ratchet rather than correct it.

## Acceptance criteria

§ 6.1
The ratchet passes with the corrected number, fails with the previous one, and fails again if a canonicalisation site is added elsewhere. The comparison is still exact.

## Handoff

§ 7.1
The ratchet only tightens, as its contract requires, and goal 09 can run the suite without a red result that has nothing to do with the change under test.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
