# Batch overview

The completed batch `20260811T200821Z-clean-current-iterative` benchmarked 1 revision. One revision completed successfully with worker exit code 0, accepted status, passing plan validation, passing structural validation, and passing process audit. Tainted revisions: 0. Revisions lacking usable telemetry: 0.

The requested top-level files `/home/mdibbets/git/ai-skills/benchmark/results/20260811T200821Z-clean-current-iterative/analysis/benchmark-test.md` and `/home/mdibbets/git/ai-skills/benchmark/results/20260811T200821Z-clean-current-iterative/analysis/harness-summary.tsv` were not present in the analyzer workspace; this report uses only the archived files under `/tmp/ai-skills-capsules/20260811T200821Z-clean-current-iterative/analysis/results`.

## Revision comparison

| Revision | Protocol cohort | Worker exit | Benchmark status | Validation result | HTML/HTM artifact audit | Session UUID | Telemetry records | Token total | Accepted/tainted | Taint reasons | Tagged `planning/` skill | Review mode | Reviewer sessions | Verification passes | Lifecycle / termination | Authority | Schema | Provenance | Oracle / transformation |
|---|---:|---:|---|---|---:|---|---:|---:|---|---|---|---|---|---|---|---|---|---|---|
| current | 1.4.2 (`reviewer-optimization-1.4.2`) | 0 | accepted | pass; structural pass; process audit pass | 0 | `019ff270-bba4-7981-8e53-5cfc5a13da90` | 1 | 4304813 | accepted, not tainted | none | present | iterative | Worker-internal observed sessions: 3; harness Reviewer A: `20260811T200821Z-clean-current-iterative-current-A-1786479819761595271`; harness Reviewer B: `20260811T200821Z-clean-current-iterative-current-B-1786479973112919865` | A pass 1; B pass 1; worker-internal pass 0 | A handoff-only, exit 0, independence false; B handoff terminal, exit 0, independence true; worker-internal launch/termination events observed | passed; selected authority `reviewer-b`; no unauthorized approvals | valid | passed binding; retained hashes listed below | oracle not configured / incomplete; semantic and independent rates unavailable |

## Review and adoption state

The archived state for the current-protocol revision is preserved as machine-readable approval evidence and is not collapsed into worker exit status or validation status:

```json
{
  "current": {
    "review_completed": true,
    "plan_approved": true,
    "oracle_completed": false,
    "adoptable": false,
    "semantic_true_positive_rate": null,
    "independent_catch_rate": null,
    "seeded_denominator": 0,
    "fail_closed_reasons": [
      "MISSING_DENOMINATOR",
      "ORACLE_INCOMPLETE"
    ],
    "approval_conflict": false
  }
}
```

The revision is not adoptable because the oracle was not configured/completed, the seeded denominator is 0, and semantic/independent thresholds and rates are unavailable. The selected Reviewer B approval remains valid reviewer evidence, but it does not override the fail-closed oracle and denominator reasons.

## Provenance hashes

| Revision | Record | Archive path | SHA-256 |
|---|---|---|---|
| current | source plan | `basic-test-proof-current-20260811T200821Z-clean-current-iterative-isolated-plan` | `5bd468cd40dcb6656b54988daea244c5417b327754900322470755363b7ba669` |
| current | defective-plan manifest | unavailable | unavailable |
| current | target snapshot manifest | unavailable | unavailable |
| current | selected approval | `reviewers/20260811T200821Z-clean-current-iterative-current-B-1786479973112919865/plan/approval.json` | `1c8f651a722b256e79b0532f0d7bfc3587d149f0793a5d58ce21983e5ef1d408` |
| current | selected capsule manifest | `reviewers/20260811T200821Z-clean-current-iterative-current-B-1786479973112919865/capsule-manifest.json` | `d1545234d4669ac85d1ddb0ef5a981e28989b27900c711b96042181e779efda0` |
| current | transcript | `worker.jsonl` | `81444203012d8a39277f1c9b9696361b500c92ee1795c27fa570a076955e03c1` |
| current | lifecycle handoff | `reviewer-lifecycle.jsonl` | `743e4be5da1ee37d1fb95a99598a5527129c686094b63a82571c7cf2e86a3144` |
| current | reviewer binding | `reviewer-b-session-binding.json` | `66973117dcbc5a2c3d3bda19f17f3db08076735aecbb89eff3d5d49418f8101d` |
| current | reviewer selection | `reviewer-selection.json` | `8afdf641e1171472dc408e26f042fa70edd96ad95476854badcb4331c7192f10` |

The machine-readable aliases in `reviewer-state.json`, `protocol-metadata.json`, and `telemetry.json` agree with the nested provenance records for `source_plan_sha256`, `approval_sha256`, `transcript_sha256`, and `lifecycle_handoff_sha256`. `defective_plan_sha256` and `target_snapshot_sha256` are null with no archive paths, matching the not-configured oracle/seed state.

## Deliverable inventory by revision

| Revision | Benchmark status | Selected plan directory | Goals | Work-unit inventory items | UI stories | UI run-cache items | Testing companions | Review report files | Review findings | Bug-register files | Bug entries | Context snapshots | Validation/analysis reports | Plan directory files | Result archive files | Telemetry records | Total usage tokens | Integrity warnings |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| current | accepted | `basic-test-proof-current-20260811T200821Z-clean-current-iterative-isolated-plan` | 2 | 6 | 1 | 1 | 6 | 1 | 2 in selected plan review; 3 in selected harness Reviewer B approval | 2 | 3 in `bug-register.md`; 0 in `bugs.md` | 1 | 2 | 28 | 293 | 1 | 4304813 | No extra candidate plan directories found at revision root. Tagged `planning/` skill present. |

Counting rule: the selected plan directory is the exact `Plan:` value from `evaluation.md`. Goals are `goal.md` files below that plan. Work units are ID rows in `work-unit-inventory.md` matching IDs such as `W01` or `WU-01`, not filenames. UI stories are rows in `ui-user-stories.md`; run-cache items are files under `ui-story-runs/`. Testing companions are `*-testing.md` files in the selected plan. Review report files are review documents such as `adversarial-review.md`; review findings are `AR-*` finding records in review/approval artifacts and are counted separately. Bug-register files are `bug-register.md` and `bugs.md`; bug entries are table rows with bug/finding IDs inside those files. Validation/analysis reports are `validation.md` and `analysis-report.md`. File totals are observable `find -type f` counts in the selected plan directory and revision archive.

## Token usage progression

| Revision | Previous revision | Previous tokens | Current tokens | Delta | Change | Interpretation |
|---|---|---:|---:|---:|---|---|
| current | none | unavailable | 4304813 | comparison unavailable | comparison unavailable | First and only usable-token revision in this batch. |

Overall interpretation: this batch contains only one revision, so there is no within-batch token expansion or decrease to compute. The observed token total is usable and exact according to archived telemetry, but no progression claim is possible without a prior usable-token revision. The high token use is plausibly related to the large planning skill/context reads and iterative review/correction workflow visible in the transcript and artifacts, but this is only an observed usage association, not causation.

## Developer journey by revision

### current

The worker began by writing the session ID, reading the benchmark inputs and tagged planning skill, and constructing a planning-only durable plan for future `button-chain.html` work. The plan decomposed the future task into 2 goals and 6 work units: markup, append handler, fourth-generated-button completion, behavior test design, finished-border styling, and a future UI story verification flow.

Review activity split into two layers. The harness lifecycle records one protocol cycle with Reviewer A as handoff-only and Reviewer B as the selected independent terminal approver; it also records three worker-internal reviewer subagent sessions as observed worker activity, not harness Reviewer A/B sessions. The exact number of worker-internal review rounds and fix cycles is not recorded, but the artifacts show corrections for a click-count defect, review-lifecycle synchronization, and scaffold placeholder metadata.

The most meaningful correction was in the UI proof: an earlier four-click sequence would have created generated button 4 without pressing it, so the final plan requires five direct clicks ending on generated button 4. The worker also replaced placeholder UI rationale and progress text, updated the bug register, preserved the planning-only boundary, and reran validation after adding the analysis report.

Final evidence is strong for artifact completeness and process boundaries: validation passed with 6 work units across 2 goals, structural validation passed, process audit passed, no `.html` or `.htm` files were present, and the revision archive contains the tagged `planning/` skill. The notable limit is adoption, not worker completion: Reviewer B approval is valid, but the oracle is not configured/completed, the seeded denominator is 0, and the archived state therefore remains non-adoptable with `MISSING_DENOMINATOR` and `ORACLE_INCOMPLETE`.
