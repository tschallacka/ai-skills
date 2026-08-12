# Batch overview

Batch `20260811T130218Z-current-fresh8` benchmarked 1 revision in the 1.4.2 cohort (`protocol_id=reviewer-optimization-1.4.2`). Completed successfully: 1. Tainted: 0. Lacked usable telemetry: 0.

Legacy/non-1.4.2 archives were not present in the inspected current-run archive. Cross-cohort comparison is therefore not applicable.

## Revision comparison

| Revision | Cohort / review mode | Worker exit | Validation result | HTML/HTM artifact audit | Session UUID | Telemetry records | Token total | Benchmark status | Taint reasons | Reviewer lifecycle and approval evidence |
|---|---:|---:|---|---:|---|---:|---:|---|---|---|
| current | 1.4.2 / fresh-review | 0 | pass; structural validation pass | 0 | `019ff0ea-aa8e-7b03-8468-c5b14316e662` | 1 | 3374818 | accepted | none recorded | Harness Reviewer B: `20260811T130218Z-current-fresh8-current-B-1786454382169546898`, cycle 1, verification pass 1, handoff exit 0, `independence=true`; worker-internal sessions observed separately: `019ff0ef-f488-77c1-99b3-876033874739`, `019ff0f5-d64b-7a52-86c2-35cb8cacadda`. Selected plan review says approved; archived Reviewer B `approval.json` says `overall_plan_approval=false` with one approved AR-01 finding, so final approval evidence is conflicting but not harness-tainted by `evaluation.md`. |

## Protocol 1.4.2 lifecycle detail

| Revision | Review mode | Harness reviewer sessions | Worker-internal reviewer activity | Cycles / verification passes | Finding owners and closures | Handoff and termination | Independence status | Unresolved limits | Final fresh-review approval |
|---|---|---|---|---|---|---|---|---|---|
| current | fresh-review | Reviewer B `20260811T130218Z-current-fresh8-current-B-1786454382169546898` | Two worker-subagent sessions observed with `protocol_role=worker-internal`: `019ff0ef-f488-77c1-99b3-876033874739`, `019ff0f5-d64b-7a52-86c2-35cb8cacadda`; not counted as harness Reviewer A/B protocol sessions. | Harness lifecycle records cycle 1 and verification pass 1 for Reviewer B. Worker-internal events are cycle 0 / verification pass 0. | Selected plan `adversarial-review.md` contains AR-01 as resolved placeholder. Reviewer B `approval.json` owns one AR-01 finding and does not mark overall plan approval. Oracle terminal evidence classifies AR-01 as true positive. | Reviewer B lifecycle has `launch` then `handoff` with exit code 0. Worker-internal lifecycle has observed launch and termination events, including duplicate termination records for each worker-internal session. | Reviewer B handoff records `independence=true`; worker-internal sessions have independence unavailable/not applicable. | Oracle counts unresolved 0; review/fix cycle counts beyond the recorded lifecycle are not recorded. | Conflicting evidence: selected plan says approved; Reviewer B `approval.json` says `overall_plan_approval=false`. Harness evaluation still marks the revision accepted with no taint causes. |

## Deliverable inventory by revision

| Revision | Benchmark status | Selected plan directory | Goals | Work-unit inventory items | UI story / run-cache items | Testing companions | Review reports / finding rows | Bug-register files / bug rows | Context snapshots | Validation / analysis reports | Plan files | Result archive files | Telemetry records | Total usage tokens | Planning skill archive | Integrity warnings |
|---|---|---|---:|---:|---|---:|---|---|---:|---:|---:|---:|---:|---:|---|---|
| current | accepted | `.plans/basic-test-proof-current-20260811T130218Z-current-fresh8-isolated-plan` | 2 | 6 | 1 story / 1 run-cache file | 6 | 1 report / 1 AR row | 1 register / 0 bug rows | 3 | 2 | 34 | 270 | 1 | 3374818 | present: `planning/SKILL.md` | Extra candidate/example plan directories under archived `planning/plans/` were excluded: `benchmark-four-buttons-v120`, `benchmark-four-buttons-v130`, `benchmark-four-buttons-v130-retry`, `four-button-finished-page`, `fourth-button-completion`, `progressive-button-completion`. Exactly one selected plan directory was present under `.plans/`. |

Counting rule: plan counts use only the plan named by `evaluation.md`. Work units are counted from ID rows in `work-unit-inventory.md` matching `W01`/`WU-01` style IDs, not filenames. Review reports and bug-register files are counted as files; review finding rows and bug rows are separate row counts. Context snapshots count `context-snapshot.md` plus numbered snapshot directories with `READY` markers under `context/snapshots/`. Review rounds and fix cycles are `not recorded` unless lifecycle or artifacts show explicit boundaries.

Access-audit note: this analysis used the current run instructions and archives available under the allowed analysis/result paths. The workspace-root copies of `benchmark-test.md` and `harness-summary.tsv` named in the prompt were absent, so the archived current-run copies under `/tmp/ai-skills-capsules/20260811T130218Z-current-fresh8/analysis/results/` were used. No attempted source-checkout, installed-skill, parent-path, or prior-run archive escape was observed during this analysis.

## Token usage progression

| Revision | Previous revision | Previous tokens | Current tokens | Delta | Percent change | Label | Interpretation |
|---|---|---:|---:|---:|---:|---|---|
| current | none | unavailable | 3374818 | comparison unavailable | comparison unavailable | comparison unavailable | This is the first and only usable-token revision in the 1.4.2 cohort, so no within-cohort progression can be computed. |

Overall interpretation: token expansion or decrease across the batch is not measurable because the batch contains one usable-token revision. The observed 3374818-token worker total is an exact UUID-matched telemetry value, but with no prior same-cohort revision it is only a single usage observation, not evidence of expansion or reduction.

## Developer journey by revision

### current

- The worker treated the task as a planning-only proof for a future standalone `button-chain.html`, read the benchmark instructions and the archived tagged `planning/` skill, then created a durable plan under the selected `.plans/...isolated-plan` directory. The plan split the future work into markup, completion styling, append behavior, fourth-generated completion, automated behavior testing, and browser story validation.
- Observable review rounds: one worker-documented earlier pending review and one harness Reviewer B lifecycle handoff are visible, but exact review-round boundaries are not fully recorded. Correction/fix cycles: not recorded; observable corrections include atomic test-scope tightening, replayable UI cache actions, white-border contrast wording, benchmark-boundary wording, and bug-feedback mutation-path coverage.
- The worker strengthened the plan after validation and review pressure by making the browser run cache a five-click visible-control sequence, adding six testing companions, recording a bug register with zero bug rows, adding context snapshots, and preserving the no-HTML/no-browser benchmark boundary.
- Final evidence: worker exit 0, final tagged validator pass with 6 work units across 2 goals, structural validation pass, process audit pass, 0 HTML/HTM files, telemetry available for UUID `019ff0ea-aa8e-7b03-8468-c5b14316e662`, and the exact tagged `planning/` skill archived. Notable constraint/failure: the selected plan says final review approved, while Reviewer B `approval.json` records `overall_plan_approval=false` with one approved AR-01 finding; the harness still recorded accepted status and no taint causes.
