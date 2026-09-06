# Step: 10-step-verify-clean-suite

## Ownership

- Goal: `02-untrack-compiled-libs`
- Work unit: `W16`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `clean-checkout suite proof`
- Subscope: `N/A`

## Objective

§ 4.1
In a scratch clean checkout (git archive of HEAD into TMPDIR): run the documented bootstrap, then ./run-tests.sh under the resource wrapper; the suite passes with the five libs rebuilt from nothing.

## Instructions

§ 5.1
Precondition: goals 01-04, 06 and 07 are merged and committed to branch HEAD, and the scratch tree is git archive of that commit (a staged-but-uncommitted tree cannot pass the ls-files assertion). Create the scratch checkout: git archive HEAD into TMPDIR. Run ./run-tests.sh under resource-limited-testing/scripts/limited-run.sh. The suite must pass from nothing: bootstrap rebuilds the five libs, generates REVIEWER.md when missing, and runs bootstrap.sh for rjq; the lib test proves determinism.

## Acceptance criteria

§ 6.1
The full suite is green in the scratch tree; the five libs exist afterwards and are gitignored there; git status in the scratch tree lists no generated file as staged content.

## Handoff

§ 7.1
W31 repeats this proof at whole-plan level after the remaining goals land.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
