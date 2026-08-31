# Step: 03-step-derive-counts

## Ownership

- Goal: `01-engine-core`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `src/plan-overview/src/plan/derive.rs`
- Primary symbol or file scope: `derive_counts()`
- Subscope: `N/A`

## Objective

§ 4.1
Compute every count and percentage the pages present, in one place, so two surfaces cannot disagree about the same number.

## Instructions

§ 5.1
Derive goals, steps, work units, steps complete, findings total, open and resolved, resolved percentage, review depth against target, and per-goal completion. Each derived value records what it counted so a page can link the number to its enumeration.

## Acceptance criteria

§ 6.1
Each derived value equals the count of the items it names when those items are enumerated independently, and a value with no items reports zero of zero rather than a bare zero. US-30 requires the enumeration behind a number to add up to it.

## Handoff

§ 7.1
W04 consumes these for geometry; W21 and W29 render them; W30 asserts a rendered number matches its enumeration.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
