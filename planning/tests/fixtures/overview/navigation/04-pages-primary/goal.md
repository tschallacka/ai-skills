# Goal: Overview, goal and unit pages navigated by their edges

## Current state and prior-goal handoffs

§ 2.1
Depends on goals 01 and 02. Confirmed by browser discovery: the current page truncates work-unit lists at a column edge and offers no way to move from one unit to a related one. The user asked explicitly that edges navigate to edges.

## Outcome and definition of done

§ 3.1
The monitor page shows live state and what moved; goal and unit pages show one thing in full; every dependency, ownership and grading relationship is a link. Demonstrated in the browser by walking from a unit to a unit it depends on, then to something that depends on that, without returning to an index and without truncated text.

## Why this goal is needed

§ 4.1
These three pages are what a reviewer and a monitor spend their time on. Every other page is reached from one of them, so their relationship links are the navigation model of the whole artifact rather than a convenience.

## Scope

§ 5.1
In scope: the monitor overview, the goal page, the unit page, and making every dependency, ownership and grading relationship a followable link. Out of scope: findings, tests and coverage pages, which goal 05 owns, and history, which goal 06 owns.

## Affected files, systems, data, and interfaces

§ 6.1
The page modules for overview, goal and unit, plus the edge rendering they share. They read only the derived state from goal 01 and register into the routes from goal 02.

## Dependencies and handoffs

§ 7.1
Depends on goals 01 and 02. Hands to goals 05, 06 and 07 the linking convention their pages follow, and hands to goal 08 the unit route autoplay navigates to.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: render a relationship as a link to the target page rather than as restated text, so a number and the enumeration behind it can never disagree. Risk: a long identifier or path overflows its column, which is the observed defect, so text wraps rather than clipping. Edge case: a unit with no dependencies and a goal with no started units must state the absence rather than rendering an empty region, and a goal-size exception must show its recorded reason alongside the goal.

## Owned work units

§ 9.1
`W21` — The monitor page: current phase, what moved since the last state, blockers, and the derived dashboard values. Every number links to the page that explains it rather than restating it.

§ 9.2
`W22` — One goal in full: outcome, scope, its owned units as links, its testing requirement, and its handoff. No truncation of goal names anywhere.

§ 9.3
`W23` — One work unit in full: change target, type, instructions, acceptance criteria, handoff, and its edges as links in both directions, dependencies and dependents.

§ 9.4
`W24` — The edge block on a unit page: every relationship rendered as a hop, so traversal continues from the destination without returning to an index.

§ 9.5
`W25` — Pin that each primary page renders its required fields and that every edge target resolves to a real route. Fault-inject a dangling dependency id and a goal with no units.

§ 9.6
`W26` — In the browser on the 82-unit fixture, start at a unit, click to a unit it depends on, then to something depending on that, without using an index or the back button. Record each click and the resulting page.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | These pages carry the change that fixes the current defect, so each is pinned for field presence and edge resolution and verified by a browser walk. |

## Goal-size exception
