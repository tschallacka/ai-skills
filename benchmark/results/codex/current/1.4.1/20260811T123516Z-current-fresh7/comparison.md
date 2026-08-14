# Batch overview

This batch benchmarked 1 revision in the `reviewer-optimization-1.4.2` cohort. Worker execution completed successfully for 1 revision, 1 revision was tainted, and 0 revisions lacked usable telemetry. No revision is accepted because the only revision was rejected by the blinded oracle despite a zero worker exit code and passing local validation.

| Revision | Cohort / protocol | Worker exit status | Validation result | HTML/HTM artifact audit | Session UUID | Telemetry records | Token total | Accepted/tainted status | Taint reasons | Planning skill archive |
|---|---|---:|---|---|---|---:|---:|---|---|---|
| `current` | `1.4.2` / `reviewer-optimization-1.4.2` | `0` | plan validation `pass`; structural validation `pass`; process audit `pass` | `0` HTML/HTM files | `019ff0d1-ea49-72f3-98d0-10de82c57c35` | 1 | 4566456 | tainted | `BLINDED_ORACLE_FAILED`; oracle status `rejected`; oracle grade `target defect hash mismatch: SD-01`; Reviewer B found true-positive AR-07 through AR-10 and `overall_plan_approval=false` | present at `current/planning/` |

Protocol 1.4.2 lifecycle evidence for `current`:

| Evidence class | Observed evidence |
|---|---|
| Review mode | `fresh-review` |
| Harness reviewer sessions | Reviewer B session `20260811T123516Z-current-fresh7-current-B-1786452840922098834`; no harness Reviewer A session observed |
| Harness cycles / verification passes | cycle 1, verification pass 1 |
| Finding owners / closures | Reviewer B approved true-positive AR-07, AR-08, AR-09, and AR-10; no closure evidence for those findings in the archived result |
| Handoff / termination events | Reviewer B `launch` then `handoff`, exit code 0 |
| Independence status | Reviewer B `independence=true`; worker-internal reviewer independence is `null` and not protocol ownership evidence |
| Unresolved limits | No max-cycle or max-verification-pass exhaustion observed; unresolved Reviewer B findings remain |
| Final fresh-review approval | not approved; `overall_plan_approval=false` |

Worker-internal reviewer subagents were observed separately in `reviewer-lifecycle.jsonl` with sessions `019ff0d6-43cf-7b61-bd41-854fd8f37ce8`, `019ff0db-0a12-7cf1-8010-80cfca32e757`, and `019ff0dd-e93d-7540-aaf4-de135fca10fd`; these are worker activity, not harness Reviewer A/B protocol sessions. Missing or failed final fresh-review approval is treated as unavailable/tainted evidence rather than inferred from worker prose. Access-audit evidence in the worker `analysis-report.md` and `context-snapshot.md` records no intentional source root, parent directory, git history, installed skill, previous archive, browser, server, driver, or HTML inspection; no attempted escape is recorded in the archived worker evidence.

# Deliverable inventory by revision

| Revision | Benchmark status | Selected plan directory | Goals | Work-unit inventory items | UI stories / run-cache items | Testing companions | Review report files | Bug-register files / bug entries | Context snapshots | Validation / analysis reports | Total files in selected plan | Total files in result archive | Telemetry records | Total usage tokens | Integrity warnings |
|---|---|---|---:|---:|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---|
| `current` | tainted | `basic-test-proof-current-20260811T123516Z-current-fresh7-isolated-plan` | 2 | 8 | 2 UI story rows / 2 run-cache files | 8 | 1 | 1 file / 0 bug rows | 1 | 2 | 31 | 263 | 1 | 4566456 | Extra candidate plan directories are present inside the archived tagged `planning/plans/` provenance skill (`benchmark-four-buttons-v120`, `benchmark-four-buttons-v130`, `benchmark-four-buttons-v130-retry`, `four-button-finished-page`, `fourth-button-completion`, `progressive-button-completion`) and were excluded from selected-plan counts. The worker `analysis-report.md` self-reports 30 plan files, but the observable archive contains 31. |

Counting rule: goals are `goal.md` files in the selected plan; work units are `W01`-style ID rows in `work-unit-inventory.md`; UI stories are `US-01`-style rows and run caches are files in `ui-story-runs/`; testing companions are `*-testing.md`; review reports count review artifact files, not AR findings; bug registers count `bugs.md` files separately from bug-entry rows; validation/analysis reports count `validation.md` and `analysis-report.md`. Result archive file count is the observable file count before this `comparison.md` was written.

# Token usage progression

Only `current` has usable telemetry, with 1 UUID-matched record and 4566456 total usage tokens. There is no prior revision in this 1.4.2 cohort, so revision-to-revision token progression is comparison unavailable: no second usable-token revision exists.

Overall interpretation: this batch gives a single-point token observation, not a trend. The large token total is plausibly consistent with the observed planning work: multiple worker-internal review cycles, correction passes for layout/stale-button/UI-cache issues, final validation, artifact audits, and an independent fresh-review rejection. That remains an observed usage/workload relationship, not causal proof from the token number alone.

# Developer journey by revision

## `current`

The worker treated the task as a planning-only proof and built a durable selected plan for a future `button-chain.html` implementation without creating or testing any HTML file. The plan decomposed the future task into 2 goals and 8 work units covering markup, completion styling, append behavior, completion behavior, vertical layout, static source review, the main browser story, and a stale non-last-button browser story.

Observable worker-internal review activity recorded 3 fresh review cycles and 1 bounded verification pass: the first review raised missing lifecycle evidence, missing below-it layout ownership, and absent stale-button validation; the second caught static-review ordering and a placeholder UI-classification rationale; the third pushed the run caches to split each browser click into separate rows. Separately bounded fix-cycle count is not recorded, although the artifacts show corrections after those worker-internal reviews. The resulting artifacts strengthened layout ownership with W07, stale-click validation with W08/US-02, reordered static review after layout, replaced placeholder rationale, and expanded UI run caches into action-by-action readiness evidence.

Final local validation passed with `Plan validation passed: 8 work units across 2 goals`, structural validation passed, process audit passed, and the HTML audit found 0 `.html`/`.htm` files. The notable failure is external to the worker's final self-review: the 1.4.2 blinded oracle rejected the archive after harness Reviewer B identified true-positive AR-07 through AR-10, including a core contract contradiction in `plan-description.md`, duplicate W07 ownership text, stale testing-companion headings, and incomplete W07 coverage in the static-review companion.
