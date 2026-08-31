# Step: 05-step-portability-allowlist

## Ownership

- Goal: `14-removal-test-surface`
- Work unit: `W89`
- Type: `test`

## Change target

- File: `planning/tests/test-portability-contract.sh`
- Primary symbol or file scope: `python3-shipped allowlist arm`
- Subscope: `N/A`

## Objective

§ 4.1
Remove the allowlist arm that exempts overview-serve.sh from the python3-shipped rule. With the wrapper gone the exemption has nothing to exempt, and a stale allowlist arm hides the next real violation.

## Instructions

§ 5.1
In planning/tests/test-portability-contract.sh, remove the allowlist arm that exempts the deleted wrapper from the python3-shipped rule. Remove only that arm; every other exemption names a file that still exists and has its own reason.

## Acceptance criteria

§ 6.1
The rule has no exemption naming a file that is absent from the tree, the contract test passes, and reintroducing a python3 invocation in a shipped script makes the rule report it.

## Handoff

§ 7.1
Goal 14's definition of done is reached: the suite is green, no test names the removed renderer, and every retired assertion has a named replacement that has been shown to bite. Goal 09 inherits a suite whose green result means what it says.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
