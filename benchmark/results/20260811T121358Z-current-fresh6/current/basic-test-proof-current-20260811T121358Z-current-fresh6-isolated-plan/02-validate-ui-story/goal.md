# Goal: Validate the button chain UI story

## Current state and prior-goal handoffs

§ 2.1
This goal starts after 01-build-contract creates button-chain.html and completes the static acceptance review.

## Outcome and definition of done

§ 3.1
The planned browser user story proves the completed interaction: three generated appends occur one at a time, pressing the fourth generated button clears the document, and finished appears with a visible white border.

## Why this goal is needed

§ 4.1
The future behavior is user-facing and must be verified through direct browser input, but this proof only records the story and run cache.

## Scope

§ 5.1
Included is one bounded browser story for the full button chain. Excluded during this proof is actually opening the HTML file or running browser automation.

## Affected files, systems, data, and interfaces

§ 6.1
Verification target is the future rendered button-chain.html page through normal click input. No implementation target is changed by this goal.

## Dependencies and handoffs

§ 7.1
Depends on W01 through W05 from 01-build-contract. Completion hands off browser evidence, user story status, and any bug-register entries.

## Implementation approach, risks, and edge cases

§ 8.1
The browser story must click only the visible current last button in sequence and must reject states where earlier buttons append, more than one button appears per click, the text is not exactly finished, or the white border is not visible.

## Owned work units

§ 9.1
`W06` — Run the future browser story that clicks the current last button until completion and records pass or bug evidence.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | This goal is entirely verification and owns the required browser user-story proof. |

§ 9.2
`W07` — Run the future browser story that verifies clicking a no-longer-last button does not append another button.

## Goal-size exception

§ 11.1
Not required because this goal owns two verification work units.
