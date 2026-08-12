# Goal: Remove conflicting legacy fallback behavior

## Current state and prior-goal handoffs

`setup-benchmark.sh` patches historical context tests to use a home-directory
fallback and skip missing fixtures, while the current planning contract
requires explicit configuration and fail-closed errors.

## Outcome and definition of done

Benchmark execution has one explicit no-fallback policy. Historical archive
handling is either removed or isolated outside active protocol execution with a
documented, validated boundary.

## Why this goal is needed

Conflicting fallback semantics make missing evidence look successful and
undermine the explicit fixture requirement.

## Scope

Include benchmark setup behavior, documentation, and regression tests.
Exclude changing frozen historical result archives.

## Affected files, systems, data, and interfaces

W80 updates `benchmark/planning/setup-benchmark.sh` and its compatibility
contract. W81 tests missing-fixture and fallback behavior.

## Dependencies and handoffs

Depends on AR-05 and W70-W79. Hands off one fail-closed fixture policy to the
pilot release goal.

## Implementation approach, risks, and edge cases

Remove the fallback/skip path from active execution. If frozen-tag adaptation
is unavoidable, make it an explicitly non-release inspection tool with no
success status, no silent skip, and separate evidence.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Missing-fixture behavior directly affects evidence validity. |

## Owned work units

- `W80` — Remove or isolate the legacy fallback path.
- `W81` — Test strict missing-fixture behavior.
