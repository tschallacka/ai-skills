# Batch overview

This batch contains the `reviewer-optimization-1.4.2` protocol cohort only. Total revisions benchmarked: 2. Completed successfully by worker exit and validation: 2. Tainted by harness evaluation: 1. Lacked usable telemetry: 0.

No extra candidate plan directories were observed in either revision archive. The selected plan directory for each revision is the `Plan:` value in that revision's `evaluation.md`; reviewer-copy plan directories under `reviewers/` are protocol evidence and are excluded from deliverable inventory counts.

# Deliverable inventory by revision

| Revision | Benchmark status | Worker exit | Validation result | HTML/HTM artifact audit | Session UUID | Telemetry records | Token total | Accepted/tainted status | Taint reasons | Tagged `planning/` skill | Goals | Work-unit inventory items | UI stories / run-cache items | Testing companions | Review reports / findings | Bug-register files / bug rows | Context snapshots | Validation/analysis reports | Plan files | Result archive files |
|---|---|---:|---|---|---|---:|---:|---|---|---|---:|---:|---|---:|---|---|---:|---:|---:|---:|
| `1.3.1` | tainted | 0 | pass; structural pass | `html_or_htm_files=0` | `019fef61-a96b-7e61-9500-3e102df6a3da` | 1 | 1668080 | tainted | `REVIEWER_LIFECYCLE_FAILED` from `reviewer-lifecycle.jsonl`; Reviewer B lifecycle handoff exit code 65 | present, 24 files archived | 2 | 6 | 1 / 1 | 6 | 1 / 1 | 1 / 0 | 1 | 2 | 26 | 127 |
| `1.4.1` | accepted | 0 | pass; structural pass | `html_or_htm_files=0` | `019fef52-ffe8-76b3-92b6-765ce90b52a0` | 1 | 2441020 | accepted | none in `telemetry.json` / `evaluation.md` | present, 47 files archived | 2 | 6 | 1 / 1 | 6 | 1 / 2 | 1 / 1 | 1 | 2 | 27 | 154 |

Counting rule: goals are canonical `*/goal.md` files under the selected plan directory. Work units are rows whose first table cell is an ID like `W01` or `WU-01` in `work-unit-inventory.md`, not filenames. UI stories are `US-NN` table rows, run-cache items are Markdown files in `ui-story-runs/`, testing companions are `*-testing.md` files, review reports are selected-plan `adversarial-review.md` files, review findings are `AR-NN` rows in that report, bug-register files are selected-plan `bugs.md` files, and bug rows are `BUG-*` table rows. Plan file totals include every file under the selected plan directory, including stale temporary files when present; result archive totals include every observable file under that revision archive.

# Protocol 1.4.2 lifecycle evidence

| Revision | Review mode | Worker-internal reviewer activity | Harness Reviewer A session | Harness Reviewer B session | Cycles / verification passes | Handoff and termination evidence | Independence status | Finding owners and closures | Final fresh-review approval | Unresolved limits |
|---|---|---|---|---|---|---|---|---|---|---|
| `1.3.1` | iterative | worker-subagent `019fef65-b831-77c3-9802-69575db9f84b`, cycle 0, observed launch and duplicate observed termination, protocol role `worker-internal` | `20260811T053701Z-pilot-142-matrix-iterative-1.3.1-A-1786428023392127407`, launch then handoff exit 0 | `20260811T053701Z-pilot-142-matrix-iterative-1.3.1-B-1786428228543107518`, launch then handoff exit 65 | cycle 1 for A/B; lifecycle records launch with verification pass 0 and handoff with verification pass 1 | A/B handoff events present; worker-internal termination events present | A handoff records `independence=false`; B handoff records `independence=true`; worker-internal independence not applicable | Selected plan review report has `AR-01` resolved/no missing item. Reviewer A capsule records `AR-02` low placeholder-progress finding; Reviewer B `approval.json` approves `AR-01` only. | `approval.json` says overall approval true, but harness lifecycle failed because Reviewer B exited 65 | Tainted by reviewer lifecycle; Reviewer A capsule finding closure is not evidenced in selected plan |
| `1.4.1` | iterative | worker-subagent `019fef58-32f4-7bc0-8142-413b188e59b1`, cycle 0, observed launch and duplicate observed termination, protocol role `worker-internal` | `20260811T053701Z-pilot-142-matrix-iterative-1.4.1-A-1786427288307441210`, launch then handoff exit 0 | `20260811T053701Z-pilot-142-matrix-iterative-1.4.1-B-1786427449586811980`, launch then handoff exit 0 | cycle 1 for A/B; lifecycle records launch with verification pass 0 and handoff with verification pass 1 | A/B handoff events present; worker-internal termination events present | A handoff records `independence=false`; B handoff records `independence=true`; worker-internal independence not applicable | Selected plan review report has `AR-01` off-by-one and `AR-02` placeholder review both resolved. Reviewer A capsule separately records open `AR-03` stale `goal.md.tmp.3` and open `AR-04` context snapshot W01-W05/W06 mismatch. | Reviewer B `approval.json` says overall approval true and approves `AR-01`/`AR-02` | Harness accepted the revision, but reviewer-copy evidence contains open A-owned findings not reflected in selected-plan review closure |

# Token usage progression

| Current revision | Previous usable-token revision | Previous tokens | Current tokens | Delta | Change | Inventory/journey interpretation |
|---|---|---:|---:|---:|---|---|
| `1.4.1` | `1.3.1` | 1668080 | 2441020 | +772940 | 46.34% increase | Observed usage difference with plausible planning-work expansion: `1.4.1` has richer goal descriptions, an added handoff verification work unit, static-inspection framing, a bug-status row, resolved review findings for the off-by-one click sequence and placeholder review, and a larger archived tagged planning skill. This is not causal proof from token totals alone. |

Overall, token use expanded from `1.3.1` to `1.4.1`. The expansion aligns with more explicit planning and review-correction artifacts in `1.4.1`, while both revisions still produced the same top-level count of 2 goals, 6 work units, 1 UI story, and 6 testing companions.

# Developer journey by revision

## `1.3.1`

The worker recorded its session ID from `CODEX_THREAD_ID`, read the benchmark/task inputs and the tagged planning skill, and built a planning-only directory for a future `button-chain.html` implementation. The plan split the future task into four implementation-facing work units and two verification work units: markup, append behavior, finish behavior, border styling, DOM regression proof, and a browser story.

Review rounds and fix cycles are not fully recorded. The worker analysis reports an earlier validator correction where W01's prose markup scope was changed to `#button-chain-root`; the selected plan's adversarial review says a fresh secondary reviewer approved with no missing item, while the harness Reviewer A capsule later records an unresolved low-severity placeholder-progress finding. Final validation and structural validation passed, no HTML/HTM artifact was created, and process audit passed, but the revision is tainted because the 1.4.2 reviewer lifecycle failed on Reviewer B exit code 65.

## `1.4.1`

The worker again stayed within a planning-only boundary, but produced a more explicit plan: implementation is framed as markup, script, style, and handoff inspection, followed by static inspection and a five-click browser story. The plan specifically resolves the ambiguity that generated button 4 must first be appended by clicking generated button 3, then clicked as the terminal trigger.

Observable review activity includes one selected-plan adversarial review with two resolved findings: `AR-01` corrected the off-by-one click-count inconsistency across W05, US-01, the run cache, goal text, and testing companions; `AR-02` replaced a placeholder review artifact with concrete scope, findings, and verdict. Fix-cycle count is not recorded beyond those resolved review findings and the repeated saved validator passes. Final validation, structural validation, HTML/HTM audit, and process audit passed, and the harness accepted the revision, but reviewer-copy evidence still records open A-owned findings for a stale `goal.md.tmp.3` file and a context snapshot that says W01-W05 despite W06 existing.
