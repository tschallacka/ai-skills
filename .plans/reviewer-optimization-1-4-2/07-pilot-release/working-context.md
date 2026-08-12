# Working context: 07-pilot-release

## Confirmed execution facts

- Maintainer protocol correction: historical 1.3.1/1.4.1 reports are immutable
  context only. The maintainer contract explicitly forbids rerunning an older
  version or retrofitting an older report for a newer protocol. The active
  pilot target is the current working-tree 1.4.2 protocol.

- Repository tags available for the pilot are `v1.3.1` and `1.4.1`; `v1.4.1` is not present.
- The first pilot attempt (`20260810T200630Z-pilot-142b`) was stopped after Codex repeatedly timed out refreshing the configured unavailable `gpt-5.6-luna` model.
- The benchmark now accepts `CODEX_MODEL`; pilot retries use `gpt-5.5` and are resource-capped.
- The smoke retry `20260810T203455Z-pilot-smoke2` completed plan drafting and reached the independent-review phase. Its final worker/reviewer outcome is pending in the active run.
- The first smoke attempt exposed and fixed missing `CAPSULE_ROOT` export and metadata overwriting `harness-summary.tsv`.

## Handoff

The remaining release work must consume the smoke archive only after worker exit, validation, telemetry, reviewer approval, and analyzer comparison are present. Do not infer pilot adoption from a partial or timed-out archive; record unavailable metrics and fail the decision gate closed.

## 2026-08-10 iterative pilot evidence

- Run `20260810T205301Z-pilot-142-final` completed for revision `1.4.1` in
  explicit iterative mode with `CODEX_MODEL=gpt-5.5` and the resource-limited
  runner. Worker exit, validation, structural validation, process audit,
  reviewer lifecycle, and analyzer exit all passed.
- The archived run reports UUID-matched telemetry with one record and
  `3,396,100` usage tokens, zero HTML/HTM artifacts, Reviewer A and B sessions,
  one verification pass each, and final Reviewer B approval. The analyzer
  comparison is archived at
  `benchmark/results/20260810T205301Z-pilot-142-final/comparison.md`.
- A matching fresh-review control completed as
  `20260810T210953Z-pilot-142-control` for revision `1.4.1`. It produced one
  UUID-matched telemetry record with `3,419,563` tokens, passed worker,
  validation, structural, process, and harness review checks, and was analyzed
  at `benchmark/results/20260810T210953Z-pilot-142-control/comparison.md`.
- The control analyzer correctly fail-closed on incomplete protocol-labelled
  lifecycle provenance: worker-observed reviewer subagents were not fully
  represented by lifecycle owner/closure/termination events. See
  `pilot-decision.md`; adoption is not approved until lifecycle provenance and
  the seeded-defect oracle are complete.
- Machine-readable fail-closed evidence is preserved in `comparison.json` and
  `oracle-rejection.json`; these deliberately contain unavailable oracle
  metrics rather than inferred defect-detection rates.
- Post-fix control `20260810T214045Z-pilot-142-control2` completed with
  `2,960,156` tokens, accepted harness status, complete worker-internal versus
  harness lifecycle separation, and no lifecycle taint. The remaining gate is
  strictly the absent blinded seeded-defect oracle.

## Restart attempts: 2026-08-11

- The interrupted fresh matrix had a valid archived `1.4.1` result but no
  completed `1.3.1` result.
- Restart `20260811T072517Z-pilot-142-fresh-131-restart` failed before worker
  initialization because the child Codex app-server could not write its
  read-only state mount.
- Restart `20260811T072540Z-pilot-142-fresh-131-restart2` confirmed the
  environment blocker after redirecting state to `/tmp`: Responses WebSocket
  and HTTPS transport both failed with `Operation not permitted`. No plan was
  produced and no benchmark token result is usable from either attempt.

## 2026-08-11 clean iterative 1.3.1 evidence

- Run `20260811T081559Z-pilot-142-iterative-131-clean` completed for revision
  `1.3.1` with worker exit `0`, validation/structural/process audits passed,
  reviewer lifecycle passed, telemetry available, and `1,950,663` tokens.
- The four usable cohort archives are now: iterative `1.3.1` from
  `20260811T081559Z-pilot-142-iterative-131-clean`, iterative `1.4.1` from
  `20260811T053701Z-pilot-142-matrix-iterative`, fresh `1.3.1` from
  `20260811T074548Z-pilot-142-fresh-131-restart5`, and fresh `1.4.1` from
  `20260811T061045Z-pilot-142-matrix-fresh`.
- The release gate remains fail-closed because the four live archives do not
  contain a blinded seeded-defect fixture and mode-by-mode classifications.
  `oracle-rejection.json` records this as unavailable rather than inferred.
- Context-fixture tests now fail explicitly with exit 64/66 when
  `PLANNING_CONTEXT_CACHE` is missing or unavailable; no fallback path exists.

## 2026-08-11 current-protocol oracle evidence

- Current-only fresh pilot `20260811T130218Z-current-fresh8` ran revision
  `current` under protocol 1.4.2. Worker exit, validation, structural,
  process, telemetry, and analyzer checks passed; the archive is accepted by
  the harness with one UUID-matched telemetry record and `3,374,818` tokens.
- The blinded oracle seeded three defects and graded terminal Reviewer B
  evidence successfully. The report is retained at
  `benchmark/results/20260811T130218Z-current-fresh8/current/oracle.json`:
  true-positive rate `0.0`, independent-catch rate `0.0`, three false
  negatives, and one false positive. Oracle acceptance means the protocol ran
  and produced a report; it does not mean the release-quality threshold was
  met.
- The archive also contains conflicting approval evidence: the selected plan
  review text says approved, while Reviewer B `approval.json` records
  `overall_plan_approval=false`. This is a fail-closed release issue, not a
  basis for adoption.
- No 1.3.1 or 1.4.1 run was started for this current-protocol correction.
  Those archives remain immutable historical context, and no backward-
  compatibility claim is made from them.
