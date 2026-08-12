# Goal: Perform the final code-to-plan review and decision gate

## Current state and prior-goal handoffs

After implementation goals are complete, the plan needs a final comparison of
the actual code/files on disk against every planned outcome, acceptance
criterion, and exclusion. This review has not yet been formalized as a goal.

## Outcome and definition of done

A fresh final review compares the entire planning skill implementation on disk
and the plan,
identifies any gaps or out-of-scope changes, records evidence and severity,
and asks the user whether discovered issues should amend the plan or remain as
documented exceptions. The goal cannot be completed until that user decision
is recorded and any requested amendment is fully validated.

## Why this goal is needed

Passing unit tests and structural validation do not prove that the delivered
code matches the complete plan or that undocumented deviations are acceptable.

## Scope

Include the complete planning-skill source, generated files, package boundary,
and plan-file inventory comparison; work-unit acceptance review,
dependency and exclusion checks, adversarial review, and the explicit user
decision gate. Exclude silently amending the plan or declaring gaps acceptable
without the user's decision.

## Affected files, systems, data, and interfaces

W84 owns the final code-to-plan comparison and evidence report. W85 owns the
decision record and any validated plan amendment requested by the user.

## Dependencies and handoffs

Depends on all implementation goals and the release oracle gate. W84 hands
identified gaps to the user decision gate; W85 either records an accepted
exception or creates and validates new plan state.

## Implementation approach, risks, and edge cases

Compare tracked and created files, package boundaries, generated projections,
tests, runtime behavior, and every work-unit acceptance criterion. Classify
each discrepancy as fixed, planned amendment, accepted exception, or blocker.
Do not mark the goal complete while a material discrepancy lacks an explicit
user decision.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The final review must be evidence-backed and validator-checked. |

## Owned work units

- `W84` — Compare implementation on disk against the complete plan.
- `W85` — Record the user decision and validate any resulting amendment.
