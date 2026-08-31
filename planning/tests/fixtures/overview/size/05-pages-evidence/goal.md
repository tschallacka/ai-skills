# Goal: Findings, tests and coverage as readable evidence

## Current state and prior-goal handoffs

§ 2.1
Depends on goals 01, 02 and 04. Confirmed by browser discovery: the current page renders 20 test rows that all read see companion and lead nowhere, so a reader cannot learn what any test actually runs. The user asked that tests be visible.

## Outcome and definition of done

§ 3.1
Findings, tests and coverage each get a page whose content is the evidence itself: a test shows what it runs, its command, its status and the unit it proves, replacing the see-companion placeholder rows. Demonstrated by reading a test's actual procedure on the page and following it to the unit it grades, with no clipped work-unit lists.

## Why this goal is needed

§ 4.1
A plan is only reviewable if its evidence is readable. A finding whose surfaces are not all shown, or a test whose command and expectation are not shown, moves the reader back into the plan files, which is the failure this rebuild exists to end.

## Scope

§ 5.1
In scope: the finding page with all seven surfaces, the test page with command, expectation, required failing mutation, last result and owning unit, and the coverage page. Out of scope: the history of findings, which goal 06 owns, and the graph view of coverage, which goal 07 owns.

## Affected files, systems, data, and interfaces

§ 6.1
The finding, test and coverage page modules. They read findings, testing marks and coverage rows from the derived state and link back to the units those pages grade.

## Dependencies and handoffs

§ 7.1
Depends on goals 01, 02 and 04. Hands to goal 06 the finding identity that a superseded entry refers to, and hands to goal 09 the pages the stories exercise.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: present the evidence itself rather than a pointer to it, which is what the see companion rows failed to do. Risk: a missing field reads as an absent problem, so a test with no recorded result says so rather than appearing to have passed, and a finding with an unresolved surface names it. Edge case: an outcome with no proof must be visibly flagged as uncovered rather than rendered like a covered one.

## Owned work units

§ 9.1
`W27` — One finding in full: its evidence, impact, observed contradiction, required correction, the unit it names as a link, and its status. Nothing clipped at a column edge.

§ 9.2
`W28` — A test or verification unit showing what it actually runs: the procedure from its testing companion, the registered command, its status, and the unit it proves. This replaces the see-companion placeholder rows.

§ 9.3
`W29` — The definition-of-done coverage mapping, each required outcome beside the units that produce and prove it, with every unit id a link and no truncated lists.

§ 9.4
`W30` — Pin that a test page contains its companion procedure rather than a reference to it, and that coverage rows render every listed unit id. Fault-inject a companion with no automated-tests section.

§ 9.5
`W31` — In the browser, open a test page and read its procedure without following any link off-page, then click through to the unit it proves. Confirm no see-companion text and no clipped coverage list.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The whole point is that evidence is readable rather than referenced; a test asserts the procedure is present and a browser story confirms nothing is clipped. |

## Goal-size exception
