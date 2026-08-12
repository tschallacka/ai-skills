# Release-gate evidence

W38 and W59 were executed against the current working tree under protocol 1.4.2.

## W38 release-gate suite

The documented planning and benchmark contract suite passed on 2026-08-11:

```text
bash planning/tests/test-reviewer-projection.sh
bash planning/tests/test-plan-env.sh
bash planning/tests/test-installer-manifest.sh
bash planning/tests/test-plan-integrity-and-monitor.sh
bash planning/tests/test-plan-commands.sh
bash planning/tests/test-plan-root.sh
bash planning/tests/test-progress-helpers.sh
bash planning/tests/test-planning-context-v27-contract.sh
for t in benchmark/planning/tests/test-*.sh; do bash "$t"; done
bash -n planning/scripts/*.sh benchmark/planning/*.sh
planning/scripts/validate-plan.sh .plans/reviewer-optimization-1-4-2
git diff --check
```

Result: all focused tests passed; shell syntax passed; the validator reported
`Plan validation passed: 100 work units across 21 goals.`

The required current-protocol evidence is attributable to the following two
normalized archives:

- Iterative: `benchmark/results/20260811T152024Z-old-plan-iterative/`
- Fresh review attempt: `benchmark/results/20260811T153906Z-old-plan-fresh/`

The existing approved plan-level adversarial review remains the release review
record. The run-level protocol evidence is retained in each archive, including
`evaluation.md`, `reviewer-state.json`, `oracle.json`, telemetry, lifecycle,
and comparison outputs.

## W59 pilot thresholds

The iterative archive completed with telemetry available, seeded denominator 3,
and a terminal blinded oracle. Its measured semantic and independent catch
rates were both 0/3. The current protocol therefore correctly failed closed
with `PLAN_NOT_APPROVED`, `APPROVAL_CONFLICT`,
`SEMANTIC_THRESHOLD_FAILED`, and `INDEPENDENT_THRESHOLD_FAILED`.

The fresh-review worker produced a validated isolated plan but did not return
its independent Reviewer B terminal handoff within the bounded harness run;
therefore no final fresh-review evaluation archive was emitted. This required
metric is explicitly unavailable. The run was terminated after the bounded
wait, and adoption remains fail-closed. Missing, incompatible, or failed
metrics are not inferred or backfilled.

This evidence completes the verification work units; it does not claim that
the reviewer protocol is adoptable. Completeness and adoption are separate
gates, and the adoption gate remains fail-closed when its thresholds fail.
