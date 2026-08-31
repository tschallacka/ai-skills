# Goal: The page follows the plan directory as it changes

## Current state and prior-goal handoffs

§ 2.1
Depends on goals 03, 04, 07 and 08. The user asked for a file monitor on the plan directory. Confirmed: the plan directory is user data, so the watcher may read it but must never write into it, and dependency-free means no filesystem-notification crate is available.

## Outcome and definition of done

§ 3.1
A file monitor on the plan directory drives everything live: the served page updates without a reload, the graph animates the difference rather than redrawing, and autoplay follows whatever its mode subjects it to. Demonstrated by editing plan files on disk and observing the page follow within a bounded delay, with no reload and no lost reader position.

## Why this goal is needed

§ 4.1
Autoplay and the growing graph both need to know that something changed. Without a watcher the artifact is a snapshot, and the monitoring half of what the user asked for does not exist.

## Scope

§ 5.1
In scope: detecting changes under the plan directory, coalescing them into change events, streaming them to connected clients, and applying a change to an open page without a reload. Out of scope: what any page does with the change, which goals 07 and 08 own.

## Affected files, systems, data, and interfaces

§ 6.1
The watcher and stream modules of the binary, and the client code that applies a change to the open page. It reads the plan tree and writes nothing into it.

## Dependencies and handoffs

§ 7.1
Depends on goal 03 for the connection and goals 04, 07 and 08 for the surfaces that react. Hands to goal 08 the state stream and to goal 07 the difference the graph animates.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: a bounded mtime and size scan on a stated interval, because no notification crate is permitted and the mechanism must work on Linux, macOS and Windows alike. Risk: coalescing loses the final event of a burst, which would leave the page permanently stale, so the test asserts the count sent equals the count represented and that the last event always arrives. Edge cases: a created file and a deleted file must both be noticed rather than only modifications, one client disconnecting must not disturb another, and applying a change must preserve scroll position, expansion state and the current route.

## Owned work units

§ 9.1
`W58` — Detect changes under the plan directory and report them as coalesced change events. Dependency-free means no filesystem-notification crate: a bounded mtime and size scan over the plan tree, with the interval and the tree size both stated, is the portable mechanism across Linux, macOS and Windows.

§ 9.2
`W59` — Collapse a burst of writes into one change event so a helper rewriting several files does not produce several redraws, and state the debounce interval rather than tuning it invisibly.

§ 9.3
`W60` — Publish state changes to connected pages as a stream, so the page follows without polling the whole artifact. A client that reconnects receives the current state rather than only subsequent changes.

§ 9.4
`W61` — Apply an incoming state change to the open page: update the values in place, hand the graph its before and after for animation, and preserve scroll position, expanded sections and the selected autoplay tab.

§ 9.5
`W62` — Pin the watcher: a single edit yields one event, a burst of edits within the debounce yields one event, and a change to a file outside the plan directory yields none. Fault-inject by touching a file without changing it and requiring no event.

§ 9.6
`W63` — With the binary serving, edit a plan document through a planning helper and confirm the page follows within the stated delay, that the graph animated rather than redrew, and that scroll position and expanded state survived.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Liveness can fail by being noisy or silent, so the watcher and debounce are unit-pinned and the whole chain from a helper write to the page moving is verified. |

## Goal-size exception
