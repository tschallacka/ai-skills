# Goal: Harden cancellation, telemetry integrity, taint layers, and journeys

## Current state and prior-goal handoffs

§ 2.1
The harness has basic process cleanup, SQLite/log/rollout telemetry fallback, a single accepted/tainted status, and comparison reporting, but it does not distinguish failure layers or emit the requested structured lifecycle evidence.

## Outcome and definition of done

§ 3.1
Prevent orphaned processes, select telemetry deterministically, distinguish failure causes, capture structured telemetry, and report evidence-backed developer journeys.

## Why this goal is needed

§ 4.1
A token reduction is unacceptable if interrupts leak processes, telemetry is matched to the wrong worker, or taint causes are conflated.

## Scope

§ 5.1
Include process-group cancellation, deterministic telemetry discovery, layered taint evidence, raw JSON telemetry, and evidence-backed journey reporting with focused fixtures.

## Affected files, systems, data, and interfaces

§ 6.1
Change benchmark/planning/setup-benchmark.sh, telemetry.sh, analyzer-prompt.md, and add bounded safeguard fixtures/tests.

## Dependencies and handoffs

§ 7.1
Consume archive metadata from Goal 5 and lifecycle/capsule/context records from Goals 2–4. Hand off complete evidence required for the pilot decision.

## Implementation approach, risks, and edge cases

§ 8.1
Install traps before workers start, kill process groups and descendants, reject stale/ambiguous UUID matches, preserve raw evidence, distinguish unavailable from zero, and never infer private reasoning or missing counts.

## Owned work units

§ 9.1
`W28` — Install traps before worker launch, forward interrupts to worker/reviewer descendants, wait for cleanup, remove temporary state, and return distinct interrupted status.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal changes executable benchmark or planning behavior and owns focused tests or verification units in the inventory. |

§ 9.2
`W29` — Resolve configured/current telemetry stores system-independently, match exact worker UUIDs, reject stale or ambiguous matches, and record database path and lookup method.

§ 9.3
`W30` — Consume W60 final telemetry.json, synthesize evaluation/status and taint causes from audit evidence, and preserve raw evidence; it never treats raw or rejected telemetry as valid.

§ 9.4
`W31` — Emit validated machine-readable per-worker/reviewer telemetry with exact-versus-heuristic fields, provenance, retention paths, and lifecycle records.

§ 9.5
`W32` — Require concise per-version journeys covering review rounds, findings, fixes, validation attempts, artifact expansion, and latency/token deltas; label missing evidence unavailable.

§ 9.6
`W33` — Prove no worker/reviewer/child survives interruption, ambiguous telemetry is rejected, taint causes remain separate, and archives are not published partially.

§ 9.7
`W52` — Define the validated machine-readable schema for worker/reviewer IDs, provenance, phase boundaries, token/cache composition, counts, durations, tool volumes, validator/patch/function-call metrics, command activity, and parent/child lifecycle.

§ 9.8
`W53` — Extract exact fields into raw.jsonl, preserve provenance, mark heuristic/unavailable values, and reject malformed/stale/ambiguous source matches; it does not validate or publish final telemetry.

§ 9.9
`W54` — Test complete, partial, malformed, missing, stale, ambiguous, exact, heuristic, and rollout-fallback telemetry fixtures with provenance.

§ 9.10
`W60` — Augment W53 raw telemetry, perform the sole final schema validation, and write telemetry.json or telemetry-rejection.json with phase/lifecycle metrics.

## Goal-size exception

§ 11.1
Not applicable: this goal owns multiple work units and does not use the single-unit exception.
