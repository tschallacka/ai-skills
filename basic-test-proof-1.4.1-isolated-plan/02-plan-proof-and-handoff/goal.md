# Goal: Record UI acceptance and plan proof

## Current state and prior-goal handoffs

§ 2.1
Goal 1 defines W01 and W02 and hands W07 semantic confirmation to W03. This goal records future browser acceptance and proves the planning artifacts without executing HTML.

## Outcome and definition of done

§ 3.1
The plan contains a bounded UI story and cache, an honest user-approved execution exclusion, approved disclosed review, successful structural validation, accurate trackers, and a durable handoff.

## Why this goal is needed

§ 4.1
The implementation contract is not resumable until direct user interaction, proof ownership, planning-time safety, review findings, validator evidence, progress, and the next action are explicit.

## Scope

§ 5.1
Include W03 browser instructions, US-01 and its cache, empty bug tracking, adversarial review, validate-plan.sh proof, artifact-safety checks, progress trackers, and the 1.4.1 execution report.

§ 5.2
Exclude running the browser flow, creating or inspecting HTML, starting any browser/server/driver, changing source, and claiming excluded UI evidence as passed.

## Affected files, systems, data, and interfaces

§ 6.1
Planning targets are ui-user-stories.md US-01, ui-story-runs/US-01.md, bugs.md, adversarial-review.md, progress trackers, validation evidence, and 1.4.1-analyze.md. W03 and W05 are bounded verification flows with no implementation file.

## Dependencies and handoffs

§ 7.1
W03 consumes W07; W04 documents W03 and its exclusion; W05 follows W04 and approved review; W06 follows successful validation and reports the handoff.

## Implementation approach, risks, and edge cases

§ 8.1
Define one mouse-click story from the initial button through appended button four, cache visible readiness after each click, mark it excluded with the user-provided reason, retain an empty bug register, conduct the sequential adversarial pass, validate, audit only the new plan directory for prohibited artifacts, and report without fabricating browser evidence.

## Owned work units

§ 9.1
W03 owns the future rendered-browser flow; W04 owns US-01; W05 owns structural validation and new-directory artifact safety; W06 owns the revision report. Together they make the planning handoff independently resumable.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal owns a bounded UI flow and structural validator proof. Browser execution is explicitly excluded now; plan validation is executed during this proof. |

§ 9.2
`W04` — Record the direct-interaction UI acceptance story, explicit planning-only exclusion, cache, evidence boundary, and related verification unit.

§ 9.3
`W05` — Run structural plan validation after review approval and confirm the plan directory contains no HTML or implementation artifact.

§ 9.4
`W06` — Record revision, timestamps, worker result, validator and review findings, safety-boundary result, and concise execution handoff.

## Goal-size exception

§ 11.1
No exception is needed: this goal owns four atomic work units, within the 2–10 limit.
