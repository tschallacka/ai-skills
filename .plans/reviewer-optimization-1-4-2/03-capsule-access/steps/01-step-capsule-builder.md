# Step: 01-step-capsule-builder

## Ownership

- Goal: `03-capsule-access`
- Work unit: `W11`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `render_template()`
- Subscope: `N/A`

## Objective

§ 4.1
Build a clean per-worker capsule containing only task inputs, tagged planning skill, REVIEWER.md for reviewers, and resolved relative references; keep capsule, workspace, and archive roots separate.

## Instructions

§ 5.1
Work only on `benchmark/planning/setup-benchmark.sh`, targeting `render_template() and capsule setup`. Build a clean per-worker capsule containing only task inputs, tagged planning skill, REVIEWER.md for reviewers, and resolved relative references; keep capsule, workspace, and archive roots separate. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
