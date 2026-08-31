# Step: 04-step-verify-graph

## Ownership

- Goal: `07-graph-and-coverage-of-data`
- Work unit: `W40`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `find an orphan through the graph`
- Subscope: `N/A`

## Objective

§ 4.1
Find a structural mistake through the graph, in the browser.

## Instructions

§ 5.1
On the anomalies fixture that W93 checks in, which carries the deliberately orphaned unit, use the graph to locate the orphan, click through to it, then follow one of its edges onward. Record each interaction. An earlier version of this instruction said only a fixture containing a deliberately orphaned unit; no fixture had one and no unit created it, which is what adversarial finding AR-10 recorded.

## Acceptance criteria

§ 6.1
US-08 and US-24 pass with recorded interactions, and the orphan was found through the graph rather than by knowing its id in advance.

## Handoff

§ 7.1
Evidence that the graph earns its place rather than decorating the plan.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
