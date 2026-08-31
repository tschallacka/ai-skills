# Step: 03-step-plan-dir-synonym

## Ownership

- Goal: `14-removal-test-surface`
- Work unit: `W87`
- Type: `test`

## Change target

- File: `planning/tests/test-plan-dir-synonym.sh`
- Primary symbol or file scope: `plan-dir coverage note`
- Subscope: `N/A`

## Objective

§ 4.1
Correct the note that delegates plan-dir coverage to test-overview-serve.sh, and cover the synonym directly rather than by reference to a retired test.

## Instructions

§ 5.1
In planning/tests/test-plan-dir-synonym.sh, replace the note that delegates plan-dir coverage to the serve test with direct assertions: exercise the plan-dir flag at each entry point the synonym is claimed to cover, in this file, rather than by reference to another. Delegation by comment is what let the coverage move out of sight.

## Acceptance criteria

§ 6.1
The file asserts the synonym directly for every entry point it claims, references no other test file for its coverage, and reports the loss when the flag handling is removed from any one of those entry points.

## Handoff

§ 7.1
The synonym has one owner rather than a chain of references, so retiring any other test in future cannot silently take it with it.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
