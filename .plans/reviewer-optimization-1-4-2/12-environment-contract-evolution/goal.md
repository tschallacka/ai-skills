# Goal: Define environment-contract evolution protocol

## Current state and prior-goal handoffs

Goal 11 provides global and plan-local manifests with an allow-listed set of
variables and explicit validation. The planning skill does not yet define how
that interface changes when a future variable is added or an existing variable
is replaced.

## Outcome and definition of done

The planning skill contains a mandatory schema-evolution protocol with no
backward-compatibility layer. A future manifest change must update producer,
applicable consumers, package records, tests, and review evidence as one
validated migration; stale or mismatched manifests fail closed.

## Why this goal is needed

The manifest is executable shell metadata. Without an explicit replacement
protocol, a future variable change can silently leave producers, consumers,
package records, and tests out of sync.

## Scope

Include the skill contract, plan-level protocol, package/inventory obligations,
consumer allowlist changes, fail-closed behavior, and regression coverage.
Exclude aliases, adapters, legacy modes, inferred defaults, and automatic
migration of old manifests.

## Affected files, systems, data, and interfaces

W70 updates `planning/SKILL.md` and the plan contract. W71 extends
`planning/tests/test-plan-env.sh`. Future schema changes must update the
package manifest/map and applicable consumers as part of the same mutation.

## Dependencies and handoffs

Depends on Goal 11's manifest producer and consumer contract (W67-W69). Hands
off a replacement-only schema process to future planning-skill maintainers
and requires fresh adversarial-review evidence for each schema change.

## Implementation approach, risks, and edge cases

Define the producer allowlist as the schema authority, require exact consumer
allowlists, and reject unknown or stale state before sourcing. A variable
rename is a replacement migration: remove the old name everywhere and update
all consumers in the same validated change. Never add an alias or fallback to
make a mixed-version state appear to work.

## Affected files and handoffs

W70 updates `planning/SKILL.md`. W71 extends the manifest fixture contract and
tests the rejection of stale/unknown schema state. Future variable changes
must use this goal's protocol and obtain fresh adversarial review coverage.

## Dependencies and risks

Depends on W67-W69. The main risk is a producer/consumer drift that looks
 harmless in a local shell; tests must exercise both sides and reject stale
 manifests before sourcing.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The protocol governs executable shell metadata and must prove fail-closed schema handling. |

## Owned work units

- `W70` — Document the no-backward-compatibility manifest evolution protocol.
- `W71` — Test stale/unknown schema rejection and protocol coverage.
