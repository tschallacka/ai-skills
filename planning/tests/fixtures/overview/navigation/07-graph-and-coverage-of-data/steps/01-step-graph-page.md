# Step: 01-step-graph-page

## Ownership

- Goal: `07-graph-and-coverage-of-data`
- Work unit: `W37`
- Type: `source`

## Change target

- File: `src/plan-overview/src/pages/graph.rs`
- Primary symbol or file scope: `render_graph()`
- Subscope: `N/A`

## Objective

§ 4.1
Make the dependency graph a surface rather than a table, since ordering mistakes are invisible in text.

## Instructions

§ 5.1
Render units as nodes in dependency order with status distinguishable by shape or label as well as colour, and every edge as a link. Provide a legend whose entries isolate a status. Nodes and edges are drawn from the same derivation the unit pages use.

## Acceptance criteria

§ 6.1
Nodes appear in dependency order, status is readable without colour alone, clicking an edge or node navigates, and isolating a status explains rather than silently drops the edges to hidden nodes. US-24 and US-47 apply.

## Handoff

§ 7.1
W54 supplies positions; W55 opens detail beside it; W56 animates changes between states.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
