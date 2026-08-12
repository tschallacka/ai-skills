# Goal: Synchronize the generated reviewer projection

## Current state and prior-goal handoffs

`planning/SKILL.md` was changed after `planning/REVIEWER.md` was generated.
The projection hash is stale and the new environment-contract section is not
present in the projection.

## Outcome and definition of done

Regenerate `planning/REVIEWER.md` from the current skill, record the current
source hash, and prove the projection is reproducible and complete.

## Why this goal is needed

Reviewers using a stale projection can miss mandatory protocol changes even
when the source skill is correct.

## Scope

Include reviewer generation, source-hash verification, and regression tests.
Exclude manual edits to the generated projection.

## Affected files, systems, data, and interfaces

W72 owns `planning/REVIEWER.md` generation. W73 owns the projection/hash test.

## Dependencies and handoffs

Depends on the current `planning/SKILL.md` and hands off a synchronized
reviewer profile to all review workflows.

## Implementation approach, risks, and edge cases

Run the bundled generator, verify the embedded SHA-256 against the source,
and assert required reviewer-visible sections are present. A hash mismatch is
an error, not a warning.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | A generated reviewer projection is a safety-critical protocol artifact. |

## Owned work units

- `W72` — Regenerate the reviewer projection.
- `W73` — Test projection hash and required-section consistency.
