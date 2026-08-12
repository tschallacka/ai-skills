# Step: 04-step-analyzer-access-prompt

## Ownership

- Goal: `03-capsule-access`
- Work unit: `W14`
- Type: `docs`

## Change target

- File: `benchmark/planning/analyzer-prompt.md`
- Primary symbol or file scope: `analyzer filesystem allowlist`
- Subscope: `N/A`

## Objective

§ 4.1
Constrain analyzers to the run instructions, summary, and current result archive while preserving access-audit evidence and taint semantics.

## Instructions

§ 5.1
Work only on `benchmark/planning/analyzer-prompt.md`, targeting `analyzer filesystem allowlist`. Constrain analyzers to the run instructions, summary, and current result archive while preserving access-audit evidence and taint semantics. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
