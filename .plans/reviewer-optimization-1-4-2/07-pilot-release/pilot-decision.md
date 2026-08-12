# Pilot decision: reviewer-optimization-1-4-2

## Evidence

- Current-only fresh 1.4.2 pilot: `20260811T130218Z-current-fresh8`, revision
  `current`, worker exit `0`, final validation/structural/process audits
  passed, one UUID-matched telemetry record with `3,374,818` tokens, and
  analyzer exit `0`. The archive is at
  `benchmark/results/20260811T130218Z-current-fresh8/current/`.
- The blinded oracle completed against three seeded defects and produced a
  terminal report, but classified `0` true positives and `3` false negatives
  (`true_positive_rate=0.0`, `independent_catch_rate=0.0`). This proves the
  oracle path is operational while failing the substantive catch threshold.
- Reviewer B lifecycle evidence is present, but its archived
  `overall_plan_approval=false` conflicts with the selected plan's approved
  review text. The conflict is retained and treated as fail-closed.

- Iterative run: `20260810T205301Z-pilot-142-final`, revision `1.4.1`, worker
  exit `0`, validation/structural/process audits passed, one UUID-matched
  telemetry record, `3,396,100` tokens, and final Reviewer B approval.
- Fresh control: `20260810T210953Z-pilot-142-control`, revision `1.4.1`, worker
  exit `0`, validation/structural/process audits passed, one UUID-matched
  telemetry record, `3,419,563` tokens, and final Reviewer B approval.
- Post-fix fresh control: `20260810T214045Z-pilot-142-control2`, revision
  `1.4.1`, worker exit `0`, validation/structural/process audits passed, one
  UUID-matched telemetry record, `2,960,156` tokens, complete worker-internal
  and harness lifecycle provenance, and final Reviewer B approval.
- Historical matrix context includes a valid archived `1.4.1` result under
  `20260811T061045Z-pilot-142-matrix-fresh` and a completed accepted `1.3.1`
  result under `20260811T074548Z-pilot-142-fresh-131-restart5`; these are not
  current-protocol requirements and were not rerun.
- The corrective `1.3.1` restart required one harness fix: Reviewer B wrote
  its valid `approval.json` beside the capsule plan, while the harness only
  checked inside the plan directory. The harness now accepts and canonicalizes
  either reviewer-owned location. The earlier `restart4` archive remains
  preserved as evidence of the false taint; `restart5` is the accepted rerun.
- The accepted iterative `1.3.1` archive is
  `20260811T081559Z-pilot-142-iterative-131-clean` with `1,950,663` tokens.
  The normalized four-archive comparison is recorded in `comparison.json`;
  oracle rates remain explicitly unavailable in `oracle-rejection.json`.
- Both runs produced zero HTML/HTM artifacts and archived the tagged
  `planning/` skill. The post-fix control used 435,944 fewer tokens than the
  iterative run (`-12.83%`), an observed difference only; the runs are not a
  sufficient causal performance study.

## Protocol gate

The original historical control was tainted for incomplete lifecycle
provenance. The post-fix historical control records worker-internal subagents
separately from the harness Reviewer B session. For the current protocol, the
blinded oracle is operational, but the zero catch rate and conflicting final
approval evidence keep the release gate closed.

## Decision

**Adoption: not approved; fail closed.** The current 1.4.2 protocol and
blinded-oracle path are operational, but the current pilot caught none of the
three seeded defects and contains conflicting final-approval evidence. The
historical 1.3.1/1.4.1 archives remain immutable context only; they are not
rerun or retrofitted for this protocol. Resolve the current review/defect-catch
failures and rerun only the current protocol before adopting.
