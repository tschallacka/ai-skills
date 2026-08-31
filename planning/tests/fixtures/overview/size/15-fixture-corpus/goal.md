# Goal: The checked-in fixture corpus

## Current state and prior-goal handoffs

§ 2.1
Depends on nothing: a fixture is input, and building it first is what lets every later goal record evidence rather than promise it. Confirmed by the adversarial review, finding AR-10: about a dozen stories require a plan state that neither declared fixture has, and no work unit created any of them. The two declared fixtures are also live user plans in the plans root, which cleanup-plans.sh and remove-plan.sh can change or delete, so evidence recorded against them is not reproducible by the next reader.

## Outcome and definition of done

§ 3.1
Every state a story or a work unit needs to observe exists in a fixture checked into the repository, so evidence is reproducible and cannot be changed or deleted by a plans-root helper.

## Why this goal is needed

§ 4.1
A story whose required state does not exist anywhere is not a story, it is an intention. It passes by the reviewer agreeing it would pass, which is the failure mode the whole story table exists to prevent. Checking the states in also converts the evidence from something observed once on one machine into something anyone can reproduce.

## Scope

§ 5.1
In scope: eight fixture directories under planning/tests/fixtures/overview and one test that pins their contents. Out of scope: changing what the page does with any of these states, which the goals that own those pages already cover, and the live plans in the plans root, which are read and snapshotted but never edited.

## Affected files, systems, data, and interfaces

§ 6.1
planning/tests/fixtures/overview gains navigation, size, anomalies, evidence-gaps, complete, empty-approved, fresh and malformed-state; planning/tests/test-overview-fixtures.sh is new. Every one of those files is tracked and therefore a shipped file that skill_files() must account for; W114 registers them in the dev arm, because the manifest gate fails on any tracked file in neither arm and a checked-in fixture is easy to think of as test data rather than as something that ships. The size snapshot adds roughly 340 KB to the repository, which is the cost of making the size claim reproducible and is stated here so it is a decision rather than a surprise.

## Dependencies and handoffs

§ 7.1
Hands every later goal a named fixture per story, so a story records which state it ran against instead of naming a live plan. Hands goal 09 a story pass that can be repeated on a clean checkout, and hands W90 a fixed input for the memory ceiling.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: snapshot the two real plans first, then synthesise the six edge-case fixtures, each carrying only the anomaly it exists for so a failure names one cause. Risk: a synthetic fixture that no helper would ever produce proves the page handles an impossible state; each edge-case fixture is therefore created by the plan helpers themselves and then edited minimally, and the step records which edit made it anomalous. Risk: a snapshot drifts from the live plan it came from and the size claim quietly stops matching; the snapshot records the source plan and the date, and it is the snapshot, not the live plan, that every later measurement uses. Edge case: the malformed-state fixture is invalid by construction, so any tooling that walks the fixture tree and parses what it finds must be told to skip it.

## Owned work units

§ 9.1
`W91` — Check in a frozen snapshot of the 82-unit plan the navigation stories are recorded against, replacing the live plans-root directory as the evidence source. A live plan can be changed or deleted by a plans-root helper, so evidence recorded against it is not reproducible.

§ 9.2
`W92` — Check in a frozen snapshot of the 337 KB state that fails to render today, so the size claim and the memory ceiling are measured against a fixed input rather than a moving one.

§ 9.3
`W93` — A fixture carrying the structural edge cases no real plan happens to have: a deliberately orphaned work unit, a single-unit goal with a recorded size exception, and a goal whose testing requirement is no.

§ 9.4
`W94` — A fixture carrying the evidence edge cases: a finding with a blank work-unit cell, a finding with a gated fix key, a coverage outcome with no proving unit, and a review cycle that recorded no findings.

§ 9.5
`W95` — A fixture in which every step and every verification has passed, which is the completed state no live plan in the plans root is currently in.

§ 9.6
`W96` — Create exactly one approved-with-zero-steps fixture. It is deliberately ambiguous for mode derivation and must render as a readable empty plan with no alternative fixture or escape hatch.

§ 9.7
`W97` — A fixture with no findings, no completed steps and no review cycles, which is what a plan looks like on its first day and what the page must not present as a failure.

§ 9.8
`W98` — A fixture whose state document is truncated or invalid and whose history carries a transition with no recorded time, so the page's behaviour on damaged input is observed rather than assumed.

§ 9.9
`W99` — Pin that each fixture still carries the edge case it exists for, so a fixture edited for another reason cannot silently stop covering the story that depends on it.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The corpus is only worth having if it still carries what it claims, so W99 pins each edge case to its fixture and fails when one is edited away. Without that test a fixture corrected for an unrelated reason silently stops covering the story that depends on it, which is the same coverage-lost-quietly failure the plan already guards against elsewhere. |

## Goal-size exception
