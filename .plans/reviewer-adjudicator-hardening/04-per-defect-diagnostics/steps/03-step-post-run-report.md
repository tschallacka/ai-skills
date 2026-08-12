# Step: 03-step-post-run-report

## Ownership

- Goal: `04-per-defect-diagnostics`
- Work unit: `W11`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `N/A`
- Subscope: `N/A`

## Objective

§ 4.1
Produce and archive a deterministic post-run report showing each defect, the candidate findings considered, and the exact failed predicate; verify it is reproducible across two invocations.

## Instructions

§ 5.1
Add a deterministic post-run report step or small script that prints each defect, the considered findings, and the exact failed predicate; run it twice and compare outputs.

## Acceptance criteria

§ 6.1
Two invocations produce byte-identical output; each defect row names its failed predicate or true positive.

## Handoff

§ 7.1
The report is archived; W15 includes it in the full-suite gate.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
