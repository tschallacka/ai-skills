# Goal: Verify button chain acceptance

## Current state and prior-goal handoffs

§ 2.1
This goal starts after `01-button-chain-behavior` has produced the planned `button-chain.html` behavior. During this proof, the handoff is a plan contract only; no implementation artifact exists.

## Outcome and definition of done

§ 3.1
The future implementation has a planned direct browser story proving the required click chain and finished state.

## Why this goal is needed

§ 4.1
This goal prevents the future implementation from being accepted on structure alone. It requires direct user-facing click evidence for the exact behavior without adding a second future artifact outside the requested `button-chain.html` file.

## Scope

§ 5.1
Included: one browser-story verification flow. Excluded: creating a separate future test file, adding implementation fixes during verification, weakening the story to pass, or using console/state injection instead of direct UI clicks.

## Affected files, systems, data, and interfaces

§ 6.1
The verification target is the `US-01 browser click-chain flow`; it has no implementation file and must be executed through visible browser input later.

## Dependencies and handoffs

§ 7.1
Depends on W01-W04. The final handoff is a browser story record showing the exact click sequence and finished state evidence.

## Implementation approach, risks, and edge cases

§ 8.1
The browser story must check initial state, exact one-button append per current-last click, completion only on the fourth generated button click, exact lowercase text, and visible white border. It must use ordinary mouse clicks and record failure in `bugs.md` if any acceptance point fails.

## Owned work units

§ 9.1
`W06` — Run the direct browser story by clicking through the button chain and observing the finished state.

## Goal-size exception

§ 10.1
This goal owns one verification work unit, which is allowed as a standalone verification outcome under the goal-size exception.
