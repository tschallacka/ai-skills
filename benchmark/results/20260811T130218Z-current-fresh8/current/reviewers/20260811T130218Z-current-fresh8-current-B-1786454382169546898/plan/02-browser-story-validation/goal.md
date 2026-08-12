# Goal: Direct browser story validation

## Current state and prior-goal handoffs

§ 2.1
This goal starts only after Goal 01 hands off an implemented button-chain.html and a passing automated behavior check. During this benchmark proof the browser run remains planned and unexecuted.

## Outcome and definition of done

§ 3.1
Verify the completed button chain through a real browser click sequence and record evidence that the required story passes with no open bugs.

## Why this goal is needed

§ 4.1
The requested behavior is a user-facing click flow; direct browser validation is the acceptance contract that proves the future implementation works through visible controls.

## Scope

§ 5.1
Included: one bounded story, US-01, using direct clicks or taps on the visible initial and generated buttons through the completion state.

§ 5.2
Excluded in this planning-only proof: opening the HTML file, running a browser, serving files, taking screenshots, or marking the story passed before future execution evidence exists.

## Affected files, systems, data, and interfaces

§ 6.1
Future verification target is the rendered button-chain.html file. Plan artifacts affected during execution are ui-user-stories.md, ui-story-runs/US-01.md, and bugs.md if a story failure is found.

## Dependencies and handoffs

§ 7.1
Depends on W05 from Goal 01. If US-01 fails, execution must mark US-01 bug found, retain reproduction evidence, add new investigation, fix, and retest work units/goals, update plan-description.md, work-unit-inventory.md, progress trackers, ui-user-stories.md, ui-story-runs/US-01.md, bugs.md, testing companions, and rerun the validator before executing the new scope.

§ 7.2
If a severe blocker prevents reliable story execution, pause lower-priority work, complete the investigation/fix path first, rerun validation, then restart browser story testing from US-01.

§ 7.3
Completion handoff: US-01 has browser evidence showing finished with a visible white border against a contrasting state, and bugs.md has no unresolved bug rows.

## Implementation approach, risks, and edge cases

§ 8.1
Run the cached sequence exactly: click the initial button, then generated buttons one through four as the current last button. Check the count after each append and final completion after the fifth click.

§ 8.2
If completion happens on the third generated button or requires a sixth click, treat it as a failing story and open the bug loop instead of weakening the story.

## Owned work units

§ 9.1
`W06` — Open the implemented file in a browser and use direct clicks on visible buttons from the initial state through the fourth generated button, confirming completion evidence and no unresolved bug rows.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal is itself direct browser validation of the required user story. |

## Goal-size exception

§ 11.1
Allowed single-work-unit goal: this is a standalone verification outcome, which the skill permits when the goal is genuinely verification-only.
