# Goal: Graph traversal, and every stored field reachable

## Current state and prior-goal handoffs

§ 2.1
Depends on goals 01, 02 and 04. The user required that all stored data be graphically and logically presented, and asked that autoplay in planning state be a network animation of nodes and edges appearing. Confirmed: the state document carries edges and cycles that no current page draws.

## Outcome and definition of done

§ 3.1
A graph page presents units as nodes in dependency order with status as colour and edges as links, so orphans and cycles are visible; traversal continues from an edge to that node's own edges. Demonstrated by a check that every field overview-state.sh emits is presented on some page, and by finding a deliberately orphaned unit through the graph.

## Why this goal is needed

§ 4.1
Two things only a graph gives: the shape of the dependency order, and the anomalies in it. An orphan or a cycle is invisible in a table and obvious in a graph, and in planning state the growing graph is the primary surface the reader watches.

## Scope

§ 5.1
In scope: the graph page with a node per unit and an edge per dependency, anomaly marking, node detail on click, stable layout, the growth animation, and a check that every field the state emits is presented somewhere. Out of scope: the tier system that degrades the animation, which goal 12 owns, and the live event source, which goal 10 owns.

## Affected files, systems, data, and interfaces

§ 6.1
The graph page module, the layout, the node detail panel and the growth animation, plus the data-coverage test that enumerates every state field against the pages that present it.

## Dependencies and handoffs

§ 7.1
Depends on goals 01, 02 and 04. Hands to goal 08 the planning-mode autoplay subject, hands to goal 10 the difference the graph animates, and hands to goal 12 the animation whose cost the tiers govern.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: derive positions from the dependency order so the same input lays out the same way twice, and mark anomalies distinctly rather than leaving them to be spotted. Risk: the data-coverage test samples rather than enumerates and then proves nothing, so it enumerates fields and fails naming the one that is unpresented. Edge cases: a one-node plan and a fully connected small plan must both lay out without overlap, and a click during the animation must select the node rather than being swallowed.

## Owned work units

§ 9.1
`W37` — Units as nodes in dependency order with status as colour and every edge a link, so ordering, orphans and cycles are visible rather than inferred from a table.

§ 9.2
`W38` — Name what the graph reveals: orphaned units, cycles, and verification units with no path to what they grade. Each anomaly links to the unit it concerns.

§ 9.3
`W39` — Assert that every field the state document emits is rendered on at least one page, by enumerating the parsed field set and failing on any field no page consumes. Fault-inject by adding a field and confirming the failure names it.

§ 9.4
`W40` — In the browser, use the graph page to locate a deliberately orphaned unit in a fixture and reach it by clicking, then follow one of its edges onward.

§ 9.5
`W54` — Compute stable node positions for the dependency graph so the same plan lays out identically twice, and an added unit displaces the existing layout as little as possible. Position is derived from dependency depth and ordering rather than from a random seed.

§ 9.6
`W55` — Open the detail for a clicked node beside the graph without leaving the graph page, so a reader can inspect a unit and keep the structure in view, and can continue clicking along edges from there.

§ 9.7
`W56` — Animate the difference between two states rather than redrawing: a new node eases in, a new edge draws from its source, a moved node travels to its new position. Honour the reduced-motion preference by applying the same state change without motion.

§ 9.8
`W57` — Pin that layout is deterministic for one plan and that adding a unit moves the fewest existing nodes. Fault-inject by reordering the inventory rows and requiring identical positions.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Layout determinism, anomaly detection and the field-coverage assertion are all mechanically checkable, and the orphan hunt is verified in the browser. |

## Goal-size exception
