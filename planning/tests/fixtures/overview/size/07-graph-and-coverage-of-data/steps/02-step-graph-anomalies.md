# Step: 02-step-graph-anomalies

## Ownership

- Goal: `07-graph-and-coverage-of-data`
- Work unit: `W38`
- Type: `source`

## Change target

- File: `src/plan-overview/src/pages/graph.rs`
- Primary symbol or file scope: `render_graph_anomalies()`
- Subscope: `N/A`

## Objective

§ 4.1
Name what the graph reveals, rather than leaving a reader to spot it.

## Instructions

§ 5.1
Detect and name orphaned units, dependency cycles, and verification units with no path to what they grade. Each anomaly links to the unit concerned. An empty anomaly list is stated as none found rather than rendered as blank space.

## Acceptance criteria

§ 6.1
Each anomaly class is detected on the anomalies fixture that W93 checks in and links to the unit; a clean plan, for which the navigation fixture serves, states none found. US-08 applies, including reaching the orphan by clicking. An earlier version of this criterion said only a fixture containing it, which named no fixture and no owner.

## Handoff

§ 7.1
These are the same classes the plan validator reports, so a disagreement between page and validator is itself a defect.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
