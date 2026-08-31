# Goal: What changed, what was superseded, what was discarded and why

## Current state and prior-goal handoffs

§ 2.1
Depends on goals 01, 02 and 04. The user asked for evolvements, status, processing, discarded items and the reason for each. The plan tree records superseded findings, discarded approaches and corrected paragraphs, and none of that reaches any page today.

## Outcome and definition of done

§ 3.1
A history page presents the plan's evolution: status transitions, findings superseded and by what, work units and approaches discarded with the recorded reason, and corrected paragraphs with what they replaced. Demonstrated by reading, for one superseded finding and one discarded approach, both the current state and the reason it changed.

## Why this goal is needed

§ 4.1
A reviewer arriving late needs to know what changed and why, not just the current state. Without this page a discarded approach looks like an approach that was never considered, and the same question gets reopened.

## Scope

§ 5.1
In scope: the evolution page with status transitions and review cycles, superseded items with what replaced them, and discarded items with the recorded reason. Out of scope: the current status of anything, which goals 04 and 05 own.

## Affected files, systems, data, and interfaces

§ 6.1
The history page modules, reading status transitions, superseded findings, discarded units and corrected paragraphs from the derived state and linking to the documents they changed.

## Dependencies and handoffs

§ 7.1
Depends on goals 01, 02 and 04. Hands to goal 09 the stories that read a reason without opening a plan file.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: order most recent first, and pair every superseded or discarded item with its replacement or its reason on the same page. Risk: an item with no recorded reason silently reads as having none, so the page states that the reason is unrecorded. Edge case: a plan with no history at all must say so rather than rendering an empty timeline.

## Owned work units

§ 9.1
`W32` — The evolution page: status transitions with their timestamps, review cycles, and the current phase, ordered so the most recent change is first.

§ 9.2
`W33` — Superseded and resolved findings with what replaced them and the recorded reason, so a reader sees why a finding stopped being open rather than only that it did.

§ 9.3
`W34` — Discarded work: removed units, rejected alternatives from the approach decisions, and corrected paragraphs with what the earlier version said. The reason is presented beside the discard, never separately.

§ 9.4
`W35` — Pin that a superseded finding renders its replacement and reason, and that a rejected alternative renders its rationale. Fault-inject a discard with no recorded reason and require it to be shown as missing rather than omitted.

§ 9.5
`W36` — In the browser, find a superseded finding and read both its replacement and its reason; then find a rejected alternative and read why it was rejected. Record both.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | History is judged on whether reasons travel with their entries, including when a reason is absent, which is pinned by a fault injection and read in the browser. |

## Goal-size exception
