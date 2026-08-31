# Step: 04-step-derive-geometry

## Ownership

- Goal: `01-engine-core`
- Work unit: `W04`
- Type: `source`

## Change target

- File: `src/plan-overview/src/plan/derive.rs`
- Primary symbol or file scope: `derive_geometry()`
- Subscope: `N/A`

## Objective

§ 4.1
Derive the ring and donut geometry from the counts so the circumference and offsets exist once rather than being repeated in markup.

## Instructions

§ 5.1
Compute the donut offset and the three ring values from the derived counts, holding the circumference as a single constant. Geometry for a zero total is defined explicitly rather than emerging from a division.

## Acceptance criteria

§ 6.1
Given a known set of counts the geometry equals expected literals; a plan with zero steps produces a defined, drawable geometry rather than a divide-by-zero or an empty attribute.

## Handoff

§ 7.1
W21 renders these values; no page computes geometry of its own.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
