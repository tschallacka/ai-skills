# Goal: Effects follow the measured framerate

## Current state and prior-goal handoffs

§ 2.1
Depends on goals 07, 10 and 11 for the effects it governs. The user asked for graceful degradation of the cinematics according to frame rate. Confirmed: the largest fixture is the case where the graph animation is most likely to miss its budget.

## Outcome and definition of done

§ 3.1
The page measures its own frame times and selects a cinematic tier from a single declared table, stepping down on a sustained budget miss and back up on sustained headroom, with hysteresis. Demonstrated by throttling the machine and observing the tier fall and later recover, by confirming no information is lost at any tier, and by the reduced-motion preference pinning the minimal tier irrespective of measured speed.

## Why this goal is needed

§ 4.1
The visual ambition of goal 11 is only safe if it can retreat. Without a tier system the page either stutters on a slow machine or is designed down to the slowest machine, and both spoil what the user asked for.

## Scope

§ 5.1
In scope: the declared tier table, the frame sampler, applying a tier, and the hysteresis that stops it oscillating. Out of scope: the effects themselves, which goals 07, 10 and 11 own.

## Affected files, systems, data, and interfaces

§ 6.1
The tier table emitted by the binary and the client sampler and applier. A tier is applied as one attribute on the root so the style layer switches wholesale rather than per element.

## Dependencies and handoffs

§ 7.1
Depends on goals 07, 10 and 11. Hands to goal 09 the record of which tier was selected during the measured run.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: thresholds and the exact effects each tier disables live in one declared table rather than being tuned inside the client, so they are reviewable and testable in one place. Risk: an essential element ends up in a drop list and information is lost at a lower tier, so the test fails if anything needed to read the page is droppable. Edge cases: a brief stall must not immediately change the reported rate, an oscillating rate at a threshold must not switch repeatedly, and a reduced-motion preference pins the minimal tier irrespective of measured speed.

## Owned work units

§ 9.1
`W71` — Emit one declared table of tiers: the frame-time thresholds for stepping down and up, the hysteresis window, and exactly which effects each tier disables. Thresholds live here rather than being tuned inside the client, so they are reviewable and testable in one place.

§ 9.2
`W72` — Sample frame times over a rolling window during animation and report a sustained figure rather than a single frame, so one slow frame from an unrelated cause does not change the tier.

§ 9.3
`W73` — Apply the selected tier as one attribute on the root so the style layer switches wholesale, and state the current tier where a reader can see it. The reduced-motion preference pins the minimal tier and no measurement overrides that.

§ 9.4
`W74` — Pin the decision from the table: a sustained miss steps down one tier, sustained headroom steps up one, a value inside the hysteresis window changes nothing, and no sequence of samples can flap between tiers. Fault-inject alternating fast and slow samples and require a stable tier.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Tier selection must not flap and must never hide information, both of which are pinned by tests, with recovery verified in a real browser under throttling. |

## Goal-size exception
