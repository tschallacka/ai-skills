# Batch overview

Batch `20260810T214045Z-pilot-142-control2` benchmarked 1 revision in the `reviewer-optimization-1.4.2` protocol cohort. The single revision completed successfully: 1 accepted, 0 tainted, and 0 lacking usable telemetry.

The root-level `benchmark-test.md` and `harness-summary.tsv` named in the analyzer prompt were not present in the analysis directory; the same files were present in the current run archive under `results/` and were used. No extra candidate plan directory was observed at the revision archive root; the only selected plan counted below is the plan named by `evaluation.md`.

| Revision | Worker exit | Validation | Structural validation | HTML/HTM audit | Session UUID | Telemetry records | Token total | Benchmark status | Taint reasons | Protocol cohort | Tagged `planning/` skill |
|---|---:|---|---|---|---|---:|---:|---|---|---|---|
| `1.4.1` | 0 | pass | pass | 0 files | `019fed9e-f4f6-7053-9794-bdb0b90b000d` | 1 | 2960156 | accepted | none | `reviewer-optimization-1.4.2` | present |

# Deliverable inventory by revision

| Revision | Benchmark status | Selected plan directory | Goals | Work-unit inventory items | UI story / run-cache items | Testing companions | Review reports / findings | Bug-register files / entries | Context snapshots | Validation / analysis reports | Plan files | Result archive files | Telemetry records | Usage tokens |
|---|---|---|---:|---:|---|---:|---|---|---:|---:|---:|---:|---:|---:|
| `1.4.1` | accepted | `basic-test-proof-1.4.1-20260810T214045Z-pilot-142-control2-isolated-plan` | 2 | 7 | 1 / 1 | 8 | 1 / 1 | 1 / 1 | 1 | 2 | 29 | 126 | 1 | 2960156 |

Counting rule: goals are `goal.md` files in the selected plan directory; work units are ID rows in `work-unit-inventory.md` matching `W01`, `W02`, etc.; UI stories are `US-` rows and run-cache items are files under `ui-story-runs/`; testing companions are `*-testing.md` files; review reports are review artifact files while findings are `AR-` rows; bug-register files are `bugs.md`-style files while entries are `BUG-` rows; validation/analysis reports count `validation.md` and `analysis-report.md`.

# Token usage progression

| Current revision | Previous usable-token revision | Previous tokens | Current tokens | Delta | Change | Interpretation |
|---|---|---:|---:|---:|---|---|
| `1.4.1` | none | unavailable | 2960156 | comparison unavailable | comparison unavailable | This is the first and only usable-token revision in the 1.4.2 cohort, so no within-cohort progression can be computed. |

Overall, the batch has a single exact telemetry total and no previous usable-token revision. Any expansion or decrease claim would be unsupported; the observable result is only that the accepted `1.4.1` run consumed 2960156 tokens while producing a 29-file selected plan and a 126-file archived result.

# Protocol 1.4.2 lifecycle evidence

| Revision | Review mode | Harness reviewer sessions | Worker-internal reviewer subagents | Cycles | Verification passes | Handoff / termination events | Independence status | Finding owners / closures | Unresolved limits | Final fresh-review approval |
|---|---|---|---|---|---|---|---|---|---|---|
| `1.4.1` | fresh-review | Reviewer B session `20260810T214045Z-pilot-142-control2-1.4.1-B-1786398727756009857`; no Reviewer A protocol session recorded | Three worker-internal sessions observed: `019feda3-01b8-7c00-9037-810f6db3e19b`, `019feda5-384c-73b3-90f1-f129817339c4`, `019feda6-d4c7-7073-a646-a34efb490887` | Cycle 1 for harness Reviewer B; worker-internal cycle 0 activity separate | Harness Reviewer B pass 1; worker-internal pass 0 activity separate | Harness Reviewer B `launch` then `handoff`; worker-internal launch events plus duplicated observed termination records | Harness Reviewer B `independence=true`; worker-internal independence not applicable | Reviewer B approval records `AR-01` as `approved_resolved`; `adversarial-review.md` records no unresolved findings | none recorded | approved in reviewer `approval.json` and final lifecycle status passed |

# Developer journey by revision

## `1.4.1`

The worker treated the benchmark as a planning-only proof for future `button-chain.html` work, explicitly preserving the no-HTML, no-browser, no-server boundary. It decomposed the future task into 2 goals and 7 work units: markup, interaction logic, terminal styling, static review, browser story verification, planning artifact audit, and build-readiness review.

Review activity is recorded in two layers. Worker-internal subagents produced three observable review/fix passes: the first found five gaps, the second found the missing `analysis-report.md`, and the third verified the newly added root `button-chain-testing.md`; the harness protocol separately records a fresh Reviewer B session with cycle 1, verification pass 1, independent approval, and handoff.

The plan strengthened after review by adding progress tracking, testing companions, context snapshot, saved validator output, analysis reporting, an explicit five-click UI run cache, and an independent W06 planning-proof artifact audit. Final evidence shows tagged validation passed with 7 work units across 2 goals, structural validation passed, process audit passed, no HTML/HTM artifacts were found, and UUID-matched telemetry was available with 1 record and 2960156 tokens.
