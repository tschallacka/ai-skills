# Goal: Verify the button-chain user journey

## Current state and prior-goal handoffs

§ 2.1
This proof records future browser stories and run caches only. The future executor performs them after button-chain.html exists.

## Outcome and definition of done

§ 3.1
The future executor can prove the complete click journey through browser interaction evidence without unresolved UI bugs.

## Why this goal is needed

§ 4.1
The acceptance criteria are observable UI behavior, so direct click validation is required to prove last-button appending and the terminal completion state.

## Scope

§ 5.1
Included: US-01 incremental append verification and US-02 fourth-generated completion verification. Excluded: changing implementation during verification, using developer-tool shortcuts, serving through a web server unless the future environment requires it, or accepting screenshots without direct clicks.

## Affected files, systems, data, and interfaces

§ 6.1
Verification has no implementation file. It affects the browser evidence records in ui-user-stories.md, ui-story-runs/US-01.md, ui-story-runs/US-02.md, and bugs.md when failures are found.

## Dependencies and handoffs

§ 7.1
W05 depends on W01, W02, and W07. W06 depends on W01 through W07 so completion is tested only after the append path has produced the required fourth generated button state and the implementation contract review has passed.

## Implementation approach, risks, and edge cases

§ 8.1
Run the cached direct-click sequences in order, record pass or bug evidence in the story artifacts, and use the bug register to add investigation and fix goals before retrying any failed required story.

## Owned work units

§ 9.1
`W05` — Verify by direct browser clicks that each press on the current last button appends exactly one button below it before completion.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | This goal owns the browser story verification required for the observable UI flow. |

§ 9.2
`W06` — Verify by direct browser clicks that pressing the fourth generated button clears the document and shows exactly finished with a visible white border.

## Goal-size exception

§ 11.1
Not applicable; this goal owns two verification work units.
