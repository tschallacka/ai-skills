# Goal: Validate the button-chain user journey

## Current state and prior-goal handoffs

§ 2.1
Goal 01 supplies the future page contract; no browser execution has occurred in this proof.

## Outcome and definition of done

§ 3.1
Provide the executable browser acceptance proof for the button-chain interaction and terminal state. Definition of done is the story cache and verification companion specifying direct clicks, observable button counts, document clearing, and the exact bordered finished text.

## Why this goal is needed

§ 4.1
A separate final validation goal makes the user-visible acceptance contract executable and records direct interaction evidence without bundling fixes into verification.

## Scope

§ 5.1
In scope is US-01 only: local-file navigation, four direct clicks on the current last button, button-count assertions, terminal document state, exact text, and visible white border. Out of scope are implementation changes, console/API shortcuts, and any other story.

## Affected files, systems, data, and interfaces

§ 6.1
W04 is a bounded browser verification flow; it uses the cache at ui-story-runs/US-01.md and reports to ui-user-stories.md, bugs.md, and the analysis report.

## Dependencies and handoffs

§ 7.1
W04 depends on W01, W02, W03, and the Goal 01 contract gate W05. A future executor must run it only after the implementation goal and contract review are complete; this benchmark records it as untested because execution is forbidden here.

## Implementation approach, risks, and edge cases

§ 8.1
The flow must click the current visible last button, not a stale button reference. Record each wait and stop on the first discrepancy. Any failure follows the bug register investigation/fix/retest policy.

## Owned work units

§ 9.1
`W04` — Verify through direct mouse clicks that initial state, one-button-per-click growth through generated button 4, generated-button-4 clearing, exact finished text, contrasting background, and visible white border all match the acceptance contract.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | This goal owns the required browser verification work unit W04 and must produce direct interaction evidence. |

## Goal-size exception

§ 11.1
The single-work-unit exception applies because this is a standalone verification outcome with one bounded browser story, and the goal documents that W04 is independently demonstrable.
