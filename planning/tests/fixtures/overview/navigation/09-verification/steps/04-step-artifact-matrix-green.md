# Step: 04-step-artifact-matrix-green

## Ownership

- Goal: `09-verification`
- Work unit: `W49`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `artifact matrix is green`
- Subscope: `N/A`

## Objective

§ 4.1
Confirm the five artifacts exist and execute before any release claim.

## Instructions

§ 5.1
Confirm all five matrix jobs pass and that each executed its own artifact once, including the windows msvc leg through PowerShell. Record the run identifier as evidence.

## Acceptance criteria

§ 6.1
Five green jobs, each with an execution rather than only a build, and the run identifier recorded. A green build with no execution is explicitly not sufficient.

## Handoff

§ 7.1
This closes the no-fallback decision: the platforms claimed are the platforms proven.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
