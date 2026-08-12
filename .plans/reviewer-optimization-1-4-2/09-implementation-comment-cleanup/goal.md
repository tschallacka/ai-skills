# Goal: Remove unnecessary implementation-journey comments

## Current state and prior-goal handoffs

§ 2.1
Evaluation and implementation may require temporary developer-journey comments to record reasoning, findings, and fixes. Those comments are useful during the work but are not automatically suitable for the finished code. W38 supplies the release validation boundary and W62 supplies the execution-hygiene handoff.

## Outcome and definition of done

§ 3.1
Before the initiative is marked complete, inspect comments added or substantially changed during the work. Remove comments whose information is already clear from self-documenting code, refactor verbose journey narration into concise comments only where the behavior is not obvious, and preserve comments that explain non-obvious intent, constraints, invariants, or trade-offs.

## Why this goal is needed

§ 4.1
Developer-journey narration can make finished code noisy and obscure the actual design. A final cleanup keeps the implementation readable without losing essential context.

## Scope

§ 5.1
Include comments in implementation files changed by this initiative and comments added during evaluation or review. Exclude required plan reports, benchmark evidence, audit records, changelogs, and documentation whose purpose is to preserve the development record.

## Affected files, systems, data, and interfaces

§ 6.1
The affected implementation files are the files changed by the completed goals, identified from the final diff and work-unit handoffs. No runtime interface or behavior should change; edits are limited to comment removal or concise clarification.

## Dependencies and handoffs

§ 7.1
Depends on completed implementation/evaluation handoffs and W62. W38 consumes this goal's final comment-cleanup evidence as part of the release gate. Hand off a changed-file list, comment decisions, and final diff review to the release decision.

## Implementation approach, risks, and edge cases

§ 8.1
Review comments in context rather than deleting all comments mechanically. Keep comments that explain why code is shaped a non-obvious way, external constraints, security or compatibility requirements, invariants, or intentional trade-offs. Remove progress narration, duplicated code descriptions, and comments that became stale. Re-run relevant tests after edits and verify no behavior changes.

## Owned work units

§ 9.1
`W63` — Perform the final implementation-comment cleanup and verify that only concise, necessary comments remain in changed code.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Comment edits must be checked against the final diff and relevant tests to ensure behavior and required context are preserved. |

## Goal-size exception

§ 11.1
Allowed single-unit goal: this goal has one cohesive final cleanup outcome and one verification work unit.
