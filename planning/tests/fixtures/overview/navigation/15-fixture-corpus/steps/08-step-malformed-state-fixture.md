# Step: 08-step-malformed-state-fixture

## Ownership

- Goal: `15-fixture-corpus`
- Work unit: `W98`
- Type: `data`

## Change target

- File: `planning/tests/fixtures/overview/malformed-state`
- Primary symbol or file scope: `malformed state fixture`
- Subscope: `N/A`

## Objective

§ 4.1
A fixture whose state document is truncated or invalid and whose history carries a transition with no recorded time, so the page's behaviour on damaged input is observed rather than assumed.

## Instructions

§ 5.1
Build planning/tests/fixtures/overview/malformed-state from a valid plan, then damage it in exactly two recorded ways: truncate the state document mid-record, and remove the recorded time from one transition. Mark the directory so any tooling that walks the fixture tree and parses what it finds skips it.

## Acceptance criteria

§ 6.1
The state document is invalid in the recorded way, the transition has no time, the directory is marked as unparseable, and no other fixture-walking tool fails because of it. The page's behaviour on this input is recorded as observed, whatever it is.

## Handoff

§ 7.1
The damaged-input story has a state, so what the page does with a truncated document is a recorded observation rather than a hope.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
