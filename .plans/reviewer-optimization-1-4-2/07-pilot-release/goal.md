# Goal: Verify, package, and pilot the new protocol

## Current state and prior-goal handoffs

§ 2.1
The repository has v27 contract/oracle and installer tests plus benchmark scripts, but no 1.4.2 pilot evidence or adoption decision.

## Outcome and definition of done

§ 3.1
Add focused contract tests, update the installable v27 planning package, run the 1.4.2 pilot, and accept the protocol only when efficiency and independent defect detection are preserved.

## Why this goal is needed

§ 4.1
The protocol should ship only after structural, access, telemetry, package, and independent-review behavior are demonstrated on a bounded real run.

## Scope

§ 5.1
Include focused tests, exact package verification, immutable historical-archive analysis, one current-working-tree 1.4.2 pilot when required, comparison analysis, and final release validation. Exclude rerunning or retrofitting archived older-version reports to modern protocol requirements and exclude broad performance claims beyond compatible evidence.

## Affected files, systems, data, and interfaces

§ 6.1
Change planning/tests/test-planning-context-v27-contract.sh and test-installer-manifest.sh only for proof coverage; consume existing benchmark result archives as immutable evidence; produce new pilot artifacts only for the current 1.4.2 protocol when compatible historical evidence is insufficient.

## Dependencies and handoffs

§ 7.1
Depends on Goals 1–6. Its handoff is the final pilot decision, validation output, and package completeness evidence.

## Implementation approach, risks, and edge cases

§ 8.1
Run fresh-review control and iterative mode within comparable cohorts, compare independent defect detection as well as tokens, and fail adoption if access taint, missing telemetry, archive incompleteness, or reviewer independence occurs.

## Owned work units

§ 9.1
`W34` — Extend the v27 oracle/benchmark contract tests for capsule variables, source namespaces, checkpoint invalidation, bounded retry, and compact-read behavior.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal changes executable benchmark or planning behavior and owns focused tests or verification units in the inventory. |

§ 9.2
`W35` — Verify exact manifest coverage, destination mapping, collision/approval failure behavior, and no partial install for the 1.4.2 package.

§ 9.3
`W36` — Run the bounded 1.4.2 pilot against the current working-tree protocol with iterative mode and mandatory fresh final review, retaining complete archives and telemetry.

§ 9.4
`W37` — Compare tokens, reviewer events, findings, fixes, final validation, taint rate, independent defect detection, and archive completeness against fresh-review mode; adopt only if the decision rule passes.

§ 9.5
`W38` — Run the complete helper, manifest, contract-test, and protocol validation suite before release.

§ 9.6
`W55` — Run the same one-to-two revision matrix with default fresh-review mode, matching task, environment, artifact checks, and telemetry requirements.

§ 9.7
`W56` — Compute token and latency deltas, reviewer event/finding/fix counts, final validation, taint rate, and independent defect detection; fail adoption when evidence is unavailable or the decision rule is not met.

§ 9.8
`W57` — Require a fresh final reviewer approval, no open AR findings, complete lifecycle handoff records, and preserved Reviewer B session evidence before release validation passes.

§ 9.9
`W58` — Define a blinded seeded-defect set and calculate true positives, false negatives, independent catches, duplicates, unresolved findings, and accuracy denominators for iterative and fresh-review runs.

§ 9.10
`W59` — Run exactly one iterative and one fresh-review control for the current working-tree protocol using fixed task inputs, isolated roots, protocol metadata, and fail-closed token/latency/defect-detection thresholds.

## Goal-size exception

§ 11.1
Not applicable: this goal owns multiple work units and does not use the single-unit exception.
