# Step: 07-step-animate-growth

## Ownership

- Goal: `07-graph-and-coverage-of-data`
- Work unit: `W56`
- Type: `source`

## Change target

- File: `src/plan-overview/assets/graph.js`
- Primary symbol or file scope: `animateGrowth`
- Subscope: `N/A`

## Objective

§ 4.1
Animate the difference between two states, which is what makes planning-mode autoplay worth watching.

## Instructions

§ 5.1
Given a before and after state, ease a new node in, draw a new edge from its source, and move a moved node to its new position. Do not redraw the graph. Under the reduced-motion preference apply the same state change with no motion. Respect the cinematic tier for effect richness.

## Acceptance criteria

§ 6.1
A unit added on disk appears as an eased-in node with its edge drawn, existing nodes barely move, and the reduced-motion path applies the change without motion while keeping the new node identifiable. US-67, US-68 and US-69 apply.

## Handoff

§ 7.1
W61 hands this the before and after; W69 measures the frame budget of this animation.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
