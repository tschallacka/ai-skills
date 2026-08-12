# Goal: Verify button-chain behavior

## Current state and prior-goal handoffs

§ 2.1
This goal begins after the future implementation file exists. During this planning proof, all verification steps remain planned instructions only and are not executed.

## Outcome and definition of done

§ 3.1
Prove the future implementation satisfies the benchmark contract and separately prove this planning-only run produced no forbidden HTML artifacts. Definition of done: static review, browser story verification, and the independent planning-proof artifact audit are complete with recorded evidence.

## Why this goal is needed

§ 4.1
The benchmark behavior is observable in a browser and has edge cases that source text alone can miss. This goal produces independent proof for semantics, user interaction, and planning-only artifact hygiene.

## Scope

§ 5.1
In scope: static implementation review, one direct browser story for the button chain, and an artifact audit for this planning proof.

§ 5.2
Out of scope: changing button-chain.html, weakening acceptance criteria, using console or DOM injection as UI evidence, or testing during this planning-only benchmark run.

## Affected files, systems, data, and interfaces

§ 6.1
Verification targets are the implemented button-chain.html behavior, the US-01 browser run cache, and the isolated benchmark workspace artifact set.

## Dependencies and handoffs

§ 7.1
Depends on 01-build-button-chain for W04 and W05. W04 consumes W01-W03, W05 consumes W04, and W06 is an independent current-proof artifact audit that does not wait for future browser verification.

## Implementation approach, risks, and edge cases

§ 8.1
Run W04 before future browser testing to catch obvious semantic drift. Run US-01 only through user-facing clicks. Run W06 during this planning proof to audit the workspace for forbidden HTML artifacts and required reports. If US-01 fails after implementation, record the failure in bugs.md, add investigation and fix goals, and rerun the story after the fix rather than changing the story to match the bug.

## Owned work units

§ 9.1
`W04` — Inspect button-chain.html after implementation for exact target count semantics, append guard, terminal text, and border requirement.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | This goal consists of verification and artifact-audit units that produce the proof evidence. |

§ 9.2
`W05` — Run the direct browser user story that clicks the current last button through the fourth generated button and observes the finished state.

§ 9.3
`W06` — Confirm this planning-only proof did not create HTML/HTM artifacts and that all required planning reports are present.

## Goal-size exception

§ 11.1
N/A: this goal owns three work units, within the 2-10 work-unit limit.
