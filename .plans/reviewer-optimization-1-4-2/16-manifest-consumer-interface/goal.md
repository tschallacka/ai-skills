# Goal: Complete the explicit manifest consumer interface

## Current state and prior-goal handoffs

The consumer utility exposes `check`, `path`, and `print`; the earlier step
wording also named a direct `source` interface, although sourcing must remain
an explicit trusted-caller action.

## Outcome and definition of done

Provide one documented, validated `check` plus `path` consumer flow for
trusted helpers, with no route that sources a manifest without validation.

## Why this goal is needed

Different helpers can otherwise implement subtly different check-then-source
sequences and reintroduce long path derivation or unsafe loading.

## Scope

Include the CLI interface, documentation, helper usage, and regression tests.
Exclude automatic shell-startup sourcing.

## Affected files, systems, data, and interfaces

W78 updates `planning/scripts/plan-env.sh` and W68 documentation. W79 tests
the interface from a temporary helper.

## Dependencies and handoffs

Depends on W67-W77 and hands off one explicit interface to monitor and helper
scripts.

## Implementation approach, risks, and edge cases

Document the implemented `check` plus `path` flow and remove the misleading
direct-source wording. Ensure callers check both manifests before sourcing the
paths returned by `path`; reject missing and stale inputs.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Consumer behavior is executable and must be regression-tested. |

## Owned work units

- `W78` — Align the consumer interface and documentation.
- `W79` — Test validated helper consumption.
