# Batch overview

Total revisions benchmarked: 1. Completed successfully: 1 worker exited `0` and passed repository-local validation. Tainted revisions: 1. Revisions lacking usable telemetry: 0.

This archive contains one `reviewer-optimization-1.4.2` cohort result. No legacy cohort archive is present in this batch, so there is no within-batch legacy comparison; the revision number `1.4.1` is the benchmarked planning skill revision inside a 1.4.2 reviewer-protocol run.

| Revision | Protocol cohort | Worker exit status | Validation result | Structural validation | Process audit | HTML/HTM artifact audit | Session UUID | Telemetry records | Token total/status | Benchmark status | Accepted/tainted | Taint reasons | Archived tagged `planning/` skill | 1.4.2 lifecycle evidence |
|---|---|---:|---|---|---|---|---|---:|---:|---|---|---|---|---|
| `1.4.1` | `reviewer-optimization-1.4.2` / `1.4.2` | `0` | pass: `Plan validation passed: 8 work units across 3 goals.` | pass | pass | `html_or_htm_files=0` | `019fed62-b261-7b41-bbf5-e5bc49c82b25` | 1 | 3329509 | tainted | tainted | Unauthorized filesystem escape recorded in `analysis-report.md`: temporary helper plan under `/tmp/review-template.*`; 1.4.2 reviewer lifecycle status failed because reviewer handoff exited `65`. | present | review mode `fresh-review`; reviewer sessions recorded: 1; cycles: 1; verification passes: 1; handoff event present; termination exit `65`; independence evidence `true`; finding owner/closure records unavailable; final fresh-review approval unavailable/tainted under protocol evidence. |

# Deliverable inventory by revision

| Revision | Benchmark status | Selected plan directory | Integrity warning | Goals | Work-unit inventory items | UI story items | UI run-cache items | Testing companions | Review reports | Bug-register files | Bug entries | Context snapshots | Validation reports | Analysis reports | Plan directory files | Result archive files | Telemetry records | Total usage tokens |
|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `1.4.1` | tainted | `basic-test-proof-1.4.1-20260810T203455Z-pilot-smoke2-isolated-plan` | none; only extra top-level directory is archived tagged `planning/` skill | 3 | 8 | 1 | 1 | 8 | 1 | 1 | 0 | 1 | 1 | 1 | 32 | 94 | 1 | 3329509 |

Counting rule: goals are `*/goal.md`; work units are ID rows in `work-unit-inventory.md` matching `W01`-style IDs, not filenames; UI stories are story rows in `ui-user-stories.md`; run-cache items are files under `ui-story-runs/`; testing companions are `*-testing.md`; review reports are review-report files such as `adversarial-review.md`, counted separately from review findings; bug-register files are files such as `bugs.md`, counted separately from bug-entry rows; validation and analysis reports are `validation.md` and `analysis-report.md` in the selected plan directory. Review rounds and fix cycles are `not recorded` unless protocol lifecycle or worker events give a reproducible boundary.

# Token usage progression

| Revision | Previous usable revision | Previous tokens | Current tokens | Delta | Change | Interpretation |
|---|---|---:|---:|---:|---|---|
| `1.4.1` | comparison unavailable | unavailable | 3329509 | unavailable | comparison unavailable | This is the first and only usable-token revision in the batch, so no percentage progression can be computed. |

Overall interpretation: token usage is available for the single 1.4.2 cohort run, but the batch has no second usable revision. There is therefore no observable token expansion or decrease across revisions in this completed batch.

# Developer journey by revision

## `1.4.1`

The worker treated the task as a planning-only proof: it recorded the session UUID, read the benchmark inputs and tagged `planning/` skill from the capsule, and built a selected plan directory for a future `button-chain.html` implementation without creating an HTML/HTM artifact. The plan decomposed the future work into 3 goals and 8 work units, including implementation steps, a browser UI story, readiness review, benchmark evidence, and final artifact audit.

Observable review and correction cycles are split by evidence source. The plan artifact says Reviewer A found `AR-01`, then Reviewer B found `RB-01` and `RB-02`, and the worker strengthened the UI run cache, progress descriptions, and benchmark-evidence ownership. Under Protocol 1.4.2 lifecycle evidence, however, only one fresh-review reviewer session, one cycle, and one verification pass are recorded; finding owners/closures are unavailable in the lifecycle stream, and the reviewer handoff ended with exit `65`, so final fresh-review approval is unavailable/tainted rather than inferred from prose.

Final validation passed, structural validation passed, process audit passed, and the artifact audit found 0 `.html`/`.htm` files. The notable failure is integrity-related: the worker disclosed an unauthorized `/tmp/review-template.*` access while recovering a review template shape, and the harness marked the revision tainted despite successful planning deliverables and usable UUID-matched telemetry.
