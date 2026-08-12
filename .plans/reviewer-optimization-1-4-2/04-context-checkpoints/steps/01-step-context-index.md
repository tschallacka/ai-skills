# Step: 01-step-context-index

## Ownership

- Goal: `04-context-checkpoints`
- Work unit: `W16`
- Type: `source`

## Change target

- File: `planning/scripts/plan-context-lib.sh`
- Primary symbol or file scope: `context_build_index()`
- Subscope: `N/A`

## Objective

§ 4.1
Add source namespaces for SKILL.md, REVIEWER.md, and approved relative references, retain hash freshness checks, and invalidate memory when sources or plan files change.

## Instructions

§ 5.1
Work only on `planning/scripts/plan-context-lib.sh`, targeting `context_build_index() and context_invalidate_after_mutation()`. Add source namespaces for SKILL.md, REVIEWER.md, and approved relative references, retain hash freshness checks, and invalidate memory when sources or plan files change. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
