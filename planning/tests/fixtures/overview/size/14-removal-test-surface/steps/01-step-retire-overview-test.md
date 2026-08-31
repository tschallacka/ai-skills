# Step: 01-step-retire-overview-test

## Ownership

- Goal: `14-removal-test-surface`
- Work unit: `W85`
- Type: `test`

## Change target

- File: `planning/tests/test-plan-overview.sh`
- Primary symbol or file scope: `test-plan-overview`
- Subscope: `N/A`

## Objective

§ 4.1
Retire the twelve assertions that drive render-plan-overview.sh directly and replace them with the equivalents against the binary, so the coverage the suite had is not lost with the file it tested.

## Instructions

§ 5.1
Rewrite planning/tests/test-plan-overview.sh so each of its twelve assertions drives the binary instead of the deleted renderer. Take them one at a time and record, per assertion, what it caught and what now catches it. An assertion whose property the binary does not have is not deleted silently: state that in the step's record and raise it, because a property the old renderer guaranteed and the new one does not is a regression, not a cleanup.

## Acceptance criteria

§ 6.1
The file names the removed renderer nowhere, runs green against the binary, and carries a replacement for each of the twelve original assertions. Every replacement has been shown to fail under the fault its predecessor caught.

## Handoff

§ 7.1
W87 can cover the plan-dir synonym directly knowing which render assertions still exist to build on, and goal 09 gains the largest single block of retired coverage already accounted for.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
