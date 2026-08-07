# Goal: Record UI acceptance and plan proof

## Current state and prior-goal handoffs

§ 2.1
Goal 1 defines W01 and W02. This goal consumes those contracts to record the future browser story, preserve the planning-only exclusion, and prove the durable plan structure.

## Outcome and definition of done

§ 3.1
The plan records the bounded UI story, its authorized planning-time exclusion, validator proof, review evidence, progress state, and handoff needed for a future implementation run.

## Why this goal is needed

§ 4.1
The implementation contract is not handoff-ready until its user-visible acceptance flow, review record, validator command, no-artifact boundary, progress state, and next-executor instructions are explicit.

## Scope

§ 5.1
Include US-01, its browser run cache, the future browser verification instructions in W03, the structural validator proof in W05, review findings, progress trackers, timestamps, token evidence, and final handoff.

§ 5.2
Exclude running the future browser flow, creating or testing HTML, starting services, and changing implementation files.

## Affected files, systems, data, and interfaces

§ 6.1
Planning artifacts are ui-user-stories.md, ui-story-runs/US-01.md, bugs.md, adversarial-review.md, progress.md, and the goal progress file. W03 names a future rendered-browser flow; W05 names the local validate-plan.sh command.

## Dependencies and handoffs

§ 7.1
W03 consumes W02; W04 records the story and authorized exclusion; W05 runs after W04 and review resolution. The final handoff points a future executor to W01, W02, W03, and the user-approved exclusion boundary.

## Implementation approach, risks, and edge cases

§ 8.1
Keep the story direct-interaction based and bounded to visible button clicks. Record browser evidence as pending because the user forbids browser execution in this proof, mark the story excluded with explicit user approval, retain an empty bug register, and use validator output plus artifact inventory as current proof. Reopen review if the filename, counting rule, or browser boundary changes.

## Owned work units

§ 9.1
W03 owns the future browser flow; W04 owns the UI story artifact; W05 owns structural validation and the no-implementation-artifact check; W06 owns the durable execution report. These units complete the handoff without executing the future task.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal owns the bounded UI story and structural validator proof; both are directly verifiable, with browser execution explicitly deferred by the user's safety boundary. |

§ 9.2
`W04` — Record the direct-interaction UI acceptance story, explicit planning-only exclusion, cache, evidence boundary, and related verification unit.

§ 9.3
`W05` — Run the structural plan validator after review approval and confirm the plan contains no HTML, browser, server, or implementation artifact.

§ 9.4
`W06` — Record revision, timestamps, elapsed seconds, worker result, validator/review findings, no-artifact/process result, and token-cost evidence or explicit unavailability.

## Goal-size exception

§ 11.1
No goal-size exception: this goal has three atomic work units, within the 2–10 limit.
