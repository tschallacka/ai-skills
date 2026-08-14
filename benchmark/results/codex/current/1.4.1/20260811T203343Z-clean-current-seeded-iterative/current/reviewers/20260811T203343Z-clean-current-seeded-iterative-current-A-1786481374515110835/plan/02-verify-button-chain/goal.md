# Goal: Verify button-chain behavior

## Current state and prior-goal handoffs

§ 2.1
This goal begins only after W01 through W04 produce button-chain.html. No browser or artifact verification is run during this planning-only proof.

## Outcome and definition of done

§ 3.1
Verify the implemented HTML through a bounded browser user story and a static artifact audit. Definition of done: the user story evidence confirms the append chain, fourth-generated-button completion behavior, exact finished text, and visible white border; no open UI bugs remain.

## Why this goal is needed

§ 4.1
This goal provides independent proof that the future implementation satisfies the observable user contract and did not create unrelated artifacts.

## Scope

§ 5.1
In scope: US-01 browser verification using real clicks and a bounded artifact/process audit. Out of scope: modifying implementation files during verification and using console or injected events as passing evidence.

## Affected files, systems, data, and interfaces

§ 6.1
Verification targets are the browser flow named US-01 and the artifact audit command/output. Verification work units have File: N/A and do not edit implementation files.

## Dependencies and handoffs

§ 7.1
Depends on W01, W02, W03, W04, and W07. A failure in W05 creates a bug-register row and a new investigation/fix plan mutation before completion can proceed.

## Implementation approach, risks, and edge cases

§ 8.1
Run the cached UI story from a fresh local button-chain.html page using real clicks. Then audit generated files and lingering browser/server/driver processes. Record exact evidence in the UI story run cache, bugs.md if needed, and the goal handoff.

## Owned work units

§ 9.1
`W05` — Run the browser user story that clicks through the current-last-button chain and verifies completion output.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | This goal exists to run and record verification evidence for the user-facing task. |

§ 9.2
`W06` — Audit that only button-chain.html is created for implementation and that it contains the planned contract without unrelated execution artifacts.

## Goal-size exception

§ 11.1
Not applicable; this goal owns two verification work units, within the 2-10 work-unit limit.
