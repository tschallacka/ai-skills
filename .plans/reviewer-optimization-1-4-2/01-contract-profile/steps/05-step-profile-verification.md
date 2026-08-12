# Step: 05-step-profile-verification

## Ownership

- Goal: `01-contract-profile`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `reviewer-profile generation and contract test command`
- Subscope: `N/A`

## Objective

§ 4.1
Run generator and the focused planning contract tests against a clean temporary copy and prove the committed profile matches generated output.

## Instructions

§ 5.1
Work only on `N/A`, targeting `reviewer-profile generation and contract test command`. Run generator and the focused planning contract tests against a clean temporary copy and prove the committed profile matches generated output. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

## Acceptance criteria

§ 6.1
The named target has the required behavior, its output is bounded and reproducible, and the companion or downstream verification can observe the result without an unnamed change.

## Handoff

§ 7.1
Record the changed target, command/output evidence, and unavailable or environment-specific results for the next dependency.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
