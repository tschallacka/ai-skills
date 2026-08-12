# Goal: Verify button-chain user story

## Current state and prior-goal handoffs

§ 2.1
This goal starts only after goal 01 hands off an implemented button-chain.html with completed browser story evidence for US-01 and US-02, or a recorded bug path if either story fails. In this planning proof, the static audit is specified but not executed.

## Outcome and definition of done

§ 3.1
Run the future browser/user-story and static acceptance checks for the completed button-chain.html behavior, recording evidence that the chain and finished state satisfy the contract.

## Why this goal is needed

§ 4.1
The static audit catches acceptance-critical omissions that can be missed when a browser path passes accidentally, such as extra HTML artifacts, incorrect literal text, or missing self-contained handlers/styles.

## Scope

§ 5.1
In scope: run US-01 with normal clicks after implementation and inspect button-chain.html for the acceptance-critical contract. Out of scope: changing production code, weakening the story, using console or injected events, or creating additional HTML artifacts.

## Affected files, systems, data, and interfaces

§ 6.1
Verification reads and exercises the implemented button-chain.html. It records evidence in ui-user-stories.md, ui-story-runs/US-01.md, the relevant testing companion, and future progress trackers.

## Dependencies and handoffs

§ 7.1
W07 depends on W01 through W06 and should be run after the completion browser story. It also considers W08 evidence when checking the last-button guard contract. Its handoff is a concise evidence note that button-chain.html is the only implementation artifact and contains the expected contract pieces.

## Implementation approach, risks, and edge cases

§ 8.1
Inspect only the implemented button-chain.html and the isolated workspace artifact list. Do not create new files during verification. If the audit finds a mismatch, record it in the bug register and add investigation/fix work before claiming completion.

## Owned work units

§ 9.1
.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal owns static verification work unit W07 and records pass/fail evidence after the browser story has passed. |

§ 9.2
.

§ 9.3
`W07` — Inspect the implemented file after browser verification to confirm no extra files are required and the acceptance-critical strings, handlers, and bordered completion selector are present.

## Goal-size exception

§ 11.1
This is a standalone verification goal with one work unit, W07. The planning contract allows a single-work-unit goal for an independently demonstrable verification outcome.
