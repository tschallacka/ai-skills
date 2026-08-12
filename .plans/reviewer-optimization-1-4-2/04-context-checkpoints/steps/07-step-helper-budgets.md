# Step: 07-step-helper-budgets

## Ownership

- Goal: `04-context-checkpoints`
- Work unit: `W48`
- Type: `source`

## Change target

- File: `planning/scripts/plan-document-lib.sh`
- Primary symbol or file scope: `plan helper output mode`
- Subscope: `N/A`

## Objective

§ 4.1
Add quiet-by-default helper output, explicit verbose mode, size budgets for reports/companions/context, and bounded malformed-call retry messages.

## Instructions

§ 5.1
Work only on `planning/scripts/plan-document-lib.sh`, targeting `quiet output, size budgets, bounded retry helpers`. Add quiet-by-default helper output, explicit verbose mode, size budgets for reports/companions/context, and bounded malformed-call retry messages. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
