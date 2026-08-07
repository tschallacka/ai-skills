# Goal: Record UI acceptance and planning proof

## Current state and prior-goal handoffs

§ 2.1
Goal 1 defines W01, W02, and W07. This goal consumes their exact contract to specify the future browser proof and to establish that the planning-only artifact itself is ready and isolated.

## Outcome and definition of done

§ 3.1
US-01 and its cache define a real direct-interaction acceptance flow; the user-authorized planning-time exclusion is honest; plan review, structural and context validation, trackers, safety audit, report, and future-executor handoff are complete.

## Why this goal is needed

§ 4.1
The future code contract is not resumable until its observable UI proof, current exclusion, plan-readiness evidence, limitation disclosure, and next action are durable.

## Scope

§ 5.1
Include W03 browser instructions, W04 US-01 documentation and cache, W05 plan and context isolation checks, W06 execution report, review findings, and incomplete future-execution tracker states.

§ 5.2
Exclude executing W01 through W03, creating or inspecting HTML, launching browser, server, or driver processes, fabricating screenshots or browser results, modifying pre-existing artifacts, and claiming independent review.

## Affected files, systems, data, and interfaces

§ 6.1
Owned plan artifacts are ui-user-stories.md, ui-story-runs/US-01.md, bugs.md, adversarial-review.md, progress trackers, context snapshot files, and 1.4.1-analyze.md. W03 names one future rendered-browser flow; W05 names bounded plan-only commands.

## Dependencies and handoffs

§ 7.1
W03 depends on W02 and W07; W04 records W03 and the exclusion; adversarial review follows the complete draft; W05 follows resolved review; W06 records W05 and hands W01 as the future starting unit.

## Implementation approach, risks, and edge cases

§ 8.1
Keep US-01 bounded to five sequential interactions with the visibly current last button. Mark it excluded only for this run based on the user's express prohibition and preserve the future cache as unexecuted. Run no browser-related command. Validate plan structure and context tools under resource caps, audit only the new directory for HTML, check process state read-only, create incomplete trackers, and report exact results.

§ 8.2
If future UI validation finds a bug, set US-01 to bug found, add investigation and fix goals with new work units, update dependencies and trackers, reopen adversarial review, revalidate, and rerun US-01 after the fix. A severe blocker restarts the story from its initial state.

## Owned work units

§ 9.1
W03 owns the future browser flow.

§ 9.2
W04 owns US-01 documentation and its planning-only exclusion.

§ 9.3
W05 owns structural, bounded-context, HTML-isolation, and process-state validation.

§ 9.4
W06 owns the durable 1.4.1 execution report and concise handoff.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The bounded browser story and structural/safety validation are directly verifiable, with browser execution explicitly deferred by the user safety boundary. |

§ 9.2
`W04` — Record the direct-interaction UI acceptance story, explicit planning-only exclusion, cache, evidence boundary, and related verification unit.

§ 9.3
`W05` — Run the structural plan validator and bounded planning context checks after review, then confirm the new plan directory contains no HTML and no prohibited browser/server/driver was started by this proof.

§ 9.4
`W06` — Record revision, timing, sequential mode, review result and limitation, validator/context results, no-HTML/process result, durable inventory, and concise execution handoff.

## Goal-size exception

§ 11.1
No exception: this goal has four atomic work units, within the 2-10 limit.
