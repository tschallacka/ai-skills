# Goal: Verify button-chain user story

## Current state and prior-goal handoffs

§ 2.1
This goal depends on W01-W04 and W07 being implemented in button-chain.html, with W06 checking the integrated contract first. It has not been run during this planning-only proof.

## Outcome and definition of done

§ 3.1
A future executor verifies the full button-chain story through browser clicks, records evidence, and leaves no unresolved bugs.

## Why this goal is needed

§ 4.1
The request is user-facing, so the final acceptance proof must use direct browser interaction rather than source inspection alone.

## Scope

§ 5.1
In scope: execute US-01 through visible browser clicks in a future run and record evidence in the UI story artifacts.

§ 5.2
Out of scope: implementing fixes inside the verification step; any bug found must enter the bug feedback loop.

## Affected files, systems, data, and interfaces

§ 6.1
Verification reads the rendered future button-chain.html page and updates UI story evidence and bug records; it does not change production code.

## Dependencies and handoffs

§ 7.1
Depends on completed W01-W04, W07, and the W06 implementation contract check. A passing run hands off final acceptance evidence; a failing run hands off a bug row with investigation and fix goals added before completion.

## Implementation approach, risks, and edge cases

§ 8.1
Click the visible current last button repeatedly and observe the page through rendered UI behavior. The terminal assertion must confirm the exact lowercase text finished and a visible white border.

## Owned work units

§ 9.1
`W05` — Future browser verification performs five direct current-last-button clicks: initial, generated 1, generated 2, generated 3, and generated 4, confirming four append-producing clicks and the terminal finished state after pressing generated 4.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | This goal is a verification outcome; W05 is the direct browser proof for US-01. |

## Goal-size exception

§ 11.1
Allowed because this is a standalone verification outcome with one bounded browser-flow work unit.
