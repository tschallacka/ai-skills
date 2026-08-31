# Step: 06-step-node-detail

## Ownership

- Goal: `07-graph-and-coverage-of-data`
- Work unit: `W55`
- Type: `source`

## Change target

- File: `src/plan-overview/assets/graph.js`
- Primary symbol or file scope: `nodeDetail`
- Subscope: `N/A`

## Objective

§ 4.1
Let a reader inspect a node without losing the structure around it.

## Instructions

§ 5.1
Open the clicked node detail beside the graph without leaving the page, and allow clicking onward along that node edges from within the detail. Escape or a close control returns focus to the node.

## Acceptance criteria

§ 6.1
The graph remains visible while a detail is open, traversal can continue from the detail, and focus returns predictably on close. US-67 and US-68 apply.

## Handoff

§ 7.1
W11 peek and this detail must not both claim the same interaction; record which applies where.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
