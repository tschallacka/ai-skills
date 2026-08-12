# Batch overview

Run `20260810T210953Z-pilot-142-control` benchmarked 1 revision in the `reviewer-optimization-1.4.2` protocol cohort. 1 revision completed successfully at the worker and validator level. 1 revision is tainted for protocol-evidence completeness: the harness accepted the run, but the protocol-labelled lifecycle records only the final Reviewer B launch/handoff while the worker event log shows additional reviewer subagent sessions, and finding owner/closure plus explicit termination lifecycle events are not fully protocol-labelled. 0 revisions lacked usable telemetry.

| Revision | Protocol cohort | Worker exit status | Validation result | HTML/HTM artifact audit | Session UUID | Telemetry records | Token total/status | Benchmark status | Accepted/tainted status | Taint reasons |
|---|---|---:|---|---|---|---:|---:|---|---|---|
| `1.4.1` | `reviewer-optimization-1.4.2` / cohort `1.4.2` | 0 | pass; structural pass; process audit pass | 0 HTML/HTM files observed | `019fed82-b288-7760-8303-509ce66cf08f` | 1 | 3419563 | accepted by `evaluation.md` | tainted for protocol evidence completeness | `reviewer-lifecycle.jsonl` records only Reviewer B `launch` and `handoff`; worker JSONL shows three reviewer subagent sessions; finding owner/closure lifecycle events and explicit termination events are not fully protocol-labelled. Independence evidence is available for Reviewer B only. |

Protocol 1.4.2 lifecycle evidence for `1.4.1`: review mode `fresh-review`; lifecycle reviewer session `20260810T210953Z-pilot-142-control-1.4.1-B-1786397249981915242`; recorded cycle `1`; recorded verification pass `1`; handoff event `B-end`; independence `true`; final fresh-review approval is supported by archived `reviewers/.../plan/approval.json` with `overall_plan_approval: true`. Unresolved protocol limits: lifecycle evidence does not fully enumerate the additional worker-observed reviewer sessions, finding owners/closures, or explicit reviewer termination events.

# Deliverable inventory by revision

| Revision | Benchmark status | Selected plan directory | Goals | Work-unit inventory items | UI stories | UI run-cache items | Testing companions | Review reports | Bug-register files | Bug-register rows | Context snapshots | Validation reports | Analysis reports | Plan files | Result archive files | Telemetry records | Total usage tokens | Tagged `planning/` skill | Integrity warning |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| `1.4.1` | accepted by harness; protocol-tainted as above | `basic-test-proof-1.4.1-20260810T210953Z-pilot-142-control-isolated-plan` | 3 | 8 | 1 | 1 | 8 | 1 | 1 | 1 | 1 | 1 | 1 | 32 | 130 | 1 | 3419563 | present, 47 files under archived `planning/` | none; only the evaluation-selected plan directory was present at the archive top level |

Counting rule: plan counts use only the `Plan:` directory named in `evaluation.md`. Goals are `NN-*/goal.md` files; work units are ID rows beginning with `W01`, `W02`, etc. in `work-unit-inventory.md`; UI stories are `US-*` table rows; UI run-cache items are files under `ui-story-runs/`; testing companions are `*-testing.md` files. Review reports are counted as review-report files, bug-register files as `bugs.md`, and bug-register rows separately as `BUG-*` rows; these are not counts of review findings or bug findings.

# Token usage progression

Semantic-version order contains only `1.4.1`. There is no later revision after the first usable-token revision, so no delta or percentage comparison is available.

Overall interpretation: the batch has usable UUID-matched telemetry for its single completed revision, but it cannot support a within-batch token expansion or decrease conclusion. Any statement about token growth would require another usable-token revision in the same protocol cohort.

# Developer journey by revision

## `1.4.1`

The worker treated the benchmark as a planning-only proof, recorded the session UUID from `CODEX_THREAD_ID`, read the copied benchmark inputs and tagged planning skill, and built a durable plan for future `button-chain.html` work without creating or testing HTML. The selected plan decomposed the future page into 3 goals and 8 work units: shell/style, chain behavior, and verification/handoff, with one direct-click UI story and one cached browser run sequence left unexecuted by design.

Observable review rounds: the worker JSONL records three reviewer subagent sessions. The first reviewer found 4 issues, the second found 7 issues, and the third approved the plan with no open findings; the protocol-labelled lifecycle records only the final Reviewer B launch/handoff, so the full protocol lifecycle count is unavailable/tainted. Observable correction/fix cycles: not recorded as bounded cycles; the worker event messages show corrections to validation/report artifacts, layout ownership, verification commands, UI cache mapping, bug-loop wording, duplicate owned-work-unit formatting, and placeholder progress descriptions before final approval.

After review, the plan was strengthened with an explicit `.chain-button` vertical layout work unit, future static and browser verification ownership, a direct current-last-button UI story, a bounded run cache, a required bug investigation/fix/retest workflow, and final validation/analysis/context artifacts. Final evidence is strong at the worker-artifact level: tagged validator exit code 0, structural validation pass, process audit pass, 0 HTML/HTM artifacts, no browser/server/driver tooling left running, archived tagged `planning/` skill present, and UUID-matched telemetry with 1 record and 3419563 total usage tokens.
