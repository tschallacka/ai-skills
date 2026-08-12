# Goal: Verify and hand off the planned behavior

## Current state and prior-goal handoffs

§ 2.1
No tests or browser runs are executed during this planning-only proof. Goal 02 records the future proof targets that an executor must run after creating button-chain.html.

## Outcome and definition of done

§ 3.1
A future executor has explicit automated/manual proof steps and handoff criteria showing the button-chain behavior satisfies the task without unplanned files or hidden shortcuts.

## Why this goal is needed

§ 4.1
The task is interactive and easy to satisfy incorrectly through an off-by-one chain, missing border, or non-last-button append behavior. Separate DOM and browser proof work units make these failures observable.

## Scope

§ 5.1
In scope for future execution: one DOM interaction test target and one real browser story run. Out of scope for this proof: running those commands, opening HTML, or marking the UI story passed.

## Affected files, systems, data, and interfaces

§ 6.1
Future affected proof file is button-chain.test.js. Browser story evidence belongs in ui-story-runs/US-01.md after implementation, not during this proof.

## Dependencies and handoffs

§ 7.1
W05 depends on W01-W04. W06 depends on W01-W05 so the browser story runs after automated assertions exist. Completion handoff is the DOM test result plus US-01 browser evidence.

## Implementation approach, risks, and edge cases

§ 8.1
The DOM test must assert exact counts after each click and final computed or visible border evidence. The browser story must use real clicks only and must not use console evaluation, storage mutation, or injected events.

## Owned work units

§ 9.1
`W05` — Add an automated DOM interaction test that clicks through the chain and asserts initial, append, fourth-generated clear, exact finished text, and visible white border behavior.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | This goal owns the planned automated and browser proof work units W05 and W06. |

§ 9.2
`W06` — Run the planned user-facing browser story by clicking the current last button until the fourth generated button is pressed and record pass/fail evidence.

## Goal-size exception

§ 11.1
Not applicable; this goal owns two work units, within the 2-10 work-unit limit.
