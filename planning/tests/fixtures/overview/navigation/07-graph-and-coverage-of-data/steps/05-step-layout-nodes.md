# Step: 05-step-layout-nodes

## Ownership

- Goal: `07-graph-and-coverage-of-data`
- Work unit: `W54`
- Type: `source`

## Change target

- File: `src/plan-overview/src/pages/graph.rs`
- Primary symbol or file scope: `layout_nodes()`
- Subscope: `N/A`

## Objective

§ 4.1
Make the layout stable, so growth reads as growth rather than as a reshuffle.

## Instructions

§ 5.1
Derive node positions from dependency depth and a deterministic ordering rather than a random seed or a physics simulation, so the same plan lays out identically twice and an added unit displaces existing nodes minimally.

## Acceptance criteria

§ 6.1
The same plan produces identical positions across runs, reordering inventory rows changes nothing, and adding a unit leaves the existing nodes at or near their prior positions. US-73 applies.

## Handoff

§ 7.1
W56 animates between two of these layouts; determinism is what makes that animation meaningful.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
