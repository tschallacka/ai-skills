# Step: 01-step-verify-bootstrap-suite

## Ownership

- Goal: `05-verify-artifact-migration`
- Work unit: `W31`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `clean-checkout bootstrap and full suite`
- Subscope: `N/A`

## Objective

§ 4.1
Fresh git-archive checkout into TMPDIR: run the documented bootstrap once, then ./run-tests.sh under the resource wrapper to green; confirm git ls-files shows none of the four generated classes.

## Instructions

§ 5.1
Precondition: every prior goal is merged and committed to branch HEAD; record the commit id the archive is taken from. Fresh checkout: git archive HEAD into a TMPDIR tree. Run the documented bootstrap once (./run-tests.sh itself is the bootstrap per W08), then ./run-tests.sh under the resource wrapper to green. Assert git ls-files names none of: the rjq blob path, the five compiled libs, planning/REVIEWER.md, PORTABILITY.md.

## Acceptance criteria

§ 6.1
The full suite is green from the archive checkout; the ls-files assertion holds for all four classes; the run's summary line is captured; the scratch tree's git status shows no generated content staged or tracked.

## Handoff

§ 7.1
A green run here is the maintainer's green light for T70's release pipeline.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
