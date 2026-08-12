# Batch comparison: 20260811T112337Z-current-fresh3

## Batch overview

- Total revisions benchmarked: 1
- Completed successfully: 1
- Tainted revisions: 1
- Revisions lacking usable telemetry: 0
- Cohort: 1.4.2 only (`protocol_id=reviewer-optimization-1.4.2`)

| Revision | Worker exit status | Validation result | HTML/HTM artifact audit | Session UUID | Telemetry records | Token total / status | Benchmark status | Taint reasons |
|---|---:|---|---|---|---:|---:|---|---|
| `current` | 0 | pass; structural pass | `html_or_htm_files=0` | `019ff090-5356-7d43-a8df-2a5fa2163b6b` | 1 | 4,063,281 | tainted | `BLINDED_ORACLE_FAILED`: blinded defect seeding failed (`oracle-rejection.json`) |

## Deliverable inventory by revision

| Revision | Benchmark status | Selected plan directory | Integrity warnings | Archived tagged `planning/` skill | Goals | Work-unit inventory items | UI story items | UI run-cache items | Testing companions | Review reports | Bug-register files | Context snapshots | Validation / analysis reports | Plan files | Result archive files | Telemetry records | Usage tokens |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `current` | tainted; worker completed; validation pass | `basic-test-proof-current-20260811T112337Z-current-fresh3-isolated-plan` | none in result root; bundled sample plans under archived `planning/plans/` excluded as skill source, not candidate result plans | present (`current/planning/`, 175 files) | 2 | 8 | 2 | 2 | 8 | 1 | 1 | 1 | 2 | 31 | 262 | 1 | 4,063,281 |

Counting rule: plan counts use only the plan directory named by `evaluation.md`; work units are counted from `W01`-style ID rows in `work-unit-inventory.md`, not filenames. Review reports count report files such as `adversarial-review.md`; bug registers count register files such as `bugs.md`, separately from review findings or bug entries.

## Protocol 1.4.2 lifecycle

| Revision | Review mode | Harness reviewer sessions | Worker-internal reviewer activity | Cycles | Verification passes | Finding owners / closures | Handoff and termination events | Independence status | Unresolved limits | Final fresh-review approval |
|---|---|---|---|---|---|---|---|---|---|---|
| `current` | fresh-review | Reviewer B session `20260811T112337Z-current-fresh3-current-B-1786448284714842229`; launch `B-start`, handoff `B-end` | Worker subagent session `019ff095-4b45-7310-aef6-e3a7908ead8e`; observed launch plus duplicate observed termination records | Harness cycle 1 observed; worker-internal cycle 0 observed | Harness verification pass 1 observed; worker-internal pass 0 observed | Reviewer B approval lists AR-01 through AR-05 closed; worker plan review also marks AR-01 through AR-05 resolved | Harness handoff recorded with exit code 0; worker-internal termination observed twice with same event id | Harness Reviewer B `independence=true`; worker-internal independence unavailable/not applicable | Archive tainted by blinded oracle rejection; worker-local token evidence unavailable in `analysis-report.md`, but harness telemetry is usable | Yes, Reviewer B approval artifact present, but overall benchmark remains tainted by oracle failure |

## Token usage progression

| Revision | Previous revision | Previous tokens | Current tokens | Delta | Percent change | Label | Interpretation |
|---|---|---:|---:|---:|---:|---|---|
| `current` | none | unavailable | 4,063,281 | comparison unavailable | comparison unavailable | comparison unavailable | No prior usable-token revision exists in this 1.4.2 cohort. |

Overall interpretation: this single-revision batch establishes one usable token observation for the 1.4.2 cohort. It cannot show token expansion or decrease across revisions, and no cross-cohort token comparison is made here because legacy archives are outside the allowed current-run evidence.

## Developer journey by revision

### `current`

The worker treated the assignment as a planning-only proof: it recorded the session UUID, read the benchmark/task inputs and tagged local `planning/` skill, and built a resumable plan without creating, serving, opening, or testing any HTML file. The final plan decomposes the future `button-chain.html` task into two goals and eight inventory rows: initial markup, append logic, last-button guarding, fourth-generated completion, completion border styling, two browser-story verification units, and a static acceptance audit.

Review activity is observable in two layers. A worker-internal subagent review session is recorded separately and the plan's `adversarial-review.md` records five findings; the harness 1.4.2 lifecycle then records fresh Reviewer B launch and handoff, one harness cycle, verification pass 1, independence true, and final approval of AR-01 through AR-05. Review rounds and fix cycles beyond those lifecycle boundaries are not recorded.

The review strengthened the plan in concrete ways: it moved exact lowercase `finished` ownership into the completion source work, kept the style work focused on the white border, made the non-last-button negative browser story mandatory, added the browser-story dependency to the static audit, and corrected goal ownership/count drift. Final validation passed with `8 work units across 2 goals`, structural validation passed, process audit passed, and the HTML/HTM audit found zero files; the benchmark is nevertheless tainted because the blinded oracle rejected the run with `BLINDED_ORACLE_FAILED`.
