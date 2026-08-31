# Step: 02-step-retire-serve-test

## Ownership

- Goal: `14-removal-test-surface`
- Work unit: `W86`
- Type: `test`

## Change target

- File: `planning/tests/test-overview-serve.sh`
- Primary symbol or file scope: `test-overview-serve`
- Subscope: `N/A`

## Objective

§ 4.1
Retire the four assertions that drive overview-serve.sh and replace them with serve-mode assertions against the binary, including the port-printed-before-first-request property.

## Instructions

§ 5.1
Rewrite planning/tests/test-overview-serve.sh so its four assertions drive the binary in serve mode. Preserve the port-printed-before-first-request property explicitly: the printed port must be connectable at the moment the line is readable, which is the property that made the wrapper testable at all.

## Acceptance criteria

§ 6.1
The file names the removed wrapper nowhere, all four replacements pass against the binary, and the ordering property fails when the port line is printed before the listener accepts.

## Handoff

§ 7.1
W87 can stop delegating its plan-dir coverage to this file, because the coverage it was borrowing either exists here against the binary or is stated as gone.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
