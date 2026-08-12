# Goal: Complete manifest path and shell safety validation

## Current state and prior-goal handoffs

`plan-env.sh` validates syntax, permissions, allow-listed keys, schema version,
and two root fields, but it does not validate every derived path, ownership,
duplicate assignments, or expansion-free values.

## Outcome and definition of done

Manifest validation proves ownership, exact assignments, root containment, and
safe literal values for every generated variable before sourcing.

## Why this goal is needed

A trusted helper can otherwise source a syntactically valid manifest whose
derived paths or expansions escape the declared planning roots.

## Scope

Include validator logic and adversarial fixtures. Exclude arbitrary environment
inheritance and compatibility aliases.

## Affected files, systems, data, and interfaces

W76 updates `planning/scripts/plan-env.sh`. W77 extends its focused tests.

## Dependencies and handoffs

Depends on W67-W71 and hands off a complete fail-closed manifest boundary.

## Implementation approach, risks, and edge cases

Parse assignments exactly once, reject duplicates and unquoted expansions,
verify expected values and canonical containment for all path variables, and
check ownership in addition to mode. Validate before any source operation.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The validator protects executable shell metadata and path boundaries. |

## Owned work units

- `W76` — Harden manifest path and assignment validation.
- `W77` — Test path, ownership, duplicate, and expansion rejection.
