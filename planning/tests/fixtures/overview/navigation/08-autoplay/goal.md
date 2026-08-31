# Goal: Autoplay follows the served state, with tabs for concurrent work

## Current state and prior-goal handoffs

§ 2.1
Depends on goals 01, 02, 04 and 07. The user asked for an autoplay mode that browses to the step the agent is working on, driven by served state, with a tab per concurrent active state that vanishes when the state ends.

## Outcome and definition of done

§ 3.1
An opt-in autoplay mode navigates to the step the agent is working on, driven by the served state rather than the progress documents, and presents one tab per active state when several are open so the reader can toggle between them. Demonstrated by changing the served state and observing the page follow it, by toggling tabs, and by confirming autoplay does not move the page while the reader is browsing with it off.

## Why this goal is needed

§ 4.1
This is what makes the artifact a monitor rather than a report. Reading the progress documents would show what a tracker was last told; reading the served state shows what the agent is doing now.

## Scope

§ 5.1
In scope: deriving active states from the served state, following the active subject, the tab strip for concurrent states, and stating unavailability when there is no server. Out of scope: the transport that delivers state changes, which goal 10 owns, and the animation of the planning-mode subject, which goal 07 owns.

## Affected files, systems, data, and interfaces

§ 6.1
The autoplay client module and the tab strip. It reads the served state stream and drives the router from goal 02 rather than rendering pages of its own.

## Dependencies and handoffs

§ 7.1
Depends on goals 01, 02, 04 and 07. Hands to goal 10 the state stream it consumes, and hands to goal 09 the story that observes the page reach the unit being worked on.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: autoplay is opt-in and yields to manual interaction, saying that it has yielded, so it never fights the reader. The subject differs by mode: the active unit while implementing, the growing graph while planning. Risk: an ended state leaves an inert tab behind, so a tab vanishes when its state ends and the strip disappears when none remain. Edge case: opened from the filesystem with no server, autoplay states that it needs a served state rather than appearing engaged and doing nothing. Open question recorded: what autoplay follows in complete mode, where there is no active work.

## Owned work units

§ 9.1
`W41` — Derive the set of currently active states from the served state, not from the progress documents, so autoplay follows what the agent is doing rather than what a tracker was last told.

§ 9.2
`W42` — Navigate to the active step when autoplay is on, and keep following as the state changes. It is opt-in, its state is visible, and it never moves the page while it is off.

§ 9.3
`W43` — Present one tab per active state when several are open, let the reader toggle between them, and follow whichever is selected. A tab vanishes as soon as its state stops being active.

§ 9.4
`W44` — Pin the active-state derivation and the autoplay subject together: one active state yields one tab, several yield one tab each, a state leaving the active set removes its tab, and the subject the mode selects is the one autoplay follows, including the complete-mode case where there is none. Fault-inject a state with no active step, and a complete plan, and require the no-subject result rather than a stale one.

§ 9.5
`W45` — With the binary serving, toggle autoplay on, change the served state so a different step becomes active, and confirm the page follows. Open a second active state, toggle between tabs, then end one and confirm its tab vanishes.

§ 9.6
`W53` — Autoplay's subject is decided by the mode. In implementing mode it follows the active step. In planning mode it follows the plan being built: units appearing, dependency edges forming. In complete mode there is no active subject, so autoplay is unavailable and states why rather than offering a control that cannot move. An earlier version of this row said autoplay exists in every mode, which adversarial findings AR-08 and AR-24 recorded as contradicting US-64 and plan 8.6.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Autoplay moves the page on its own, so its derivation and tab lifecycle are unit-pinned and its live behaviour, including the off case, is verified in a browser. |

## Goal-size exception
