# Work-unit inventory: reviewer-oracle-hardening

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|
| Semantic defect manifest and private metadata validate | W01 | Goal 01 / Step 01 |
| Redacted adjudication envelope is produced | W03 | Goal 01 / Step 02 |
| Consolidated findings receive semantic one-to-many coverage scores | W02 | Goal 01 / Step 03 |
| Approval and adoption states are distinct and fail closed | W04 | Goal 02 / Step 01 |
| Reviewer contract and prompt agree on approval evidence | W05 | Goal 02 / Step 02 |
| Analyzer/report schema exposes semantic and approval metrics | W06 | Goal 02 / Step 03 |
| Oracle regression suite proves semantic consolidated coverage | W07 | Goal 03 / Step 01 |
| Approval-state regression suite rejects false adoption | W08 | Goal 03 / Step 02 |
| Blinding and redaction regression suite passes | W09 | Goal 03 / Step 03 |
| Pilot failure is pinned in a deterministic fixture | W10 | Goal 03 / Step 04 |
| Full current-protocol release gate produces the correct decision | W11 | Goal 03 / Step 05 |
| Semantic manifest schema has dedicated regression coverage | W12 | Goal 01 / Step 04 |
| Analyzer state schema has dedicated regression coverage | W13 | Goal 02 / Step 04 |
| Generated reviewer projection matches the contract | W14 | Goal 02 / Step 05 |
| Benchmark worker prompt carries the contract | W15 | Goal 02 / Step 06 |
| Protocol metadata and evaluation expose state | W16 | Goal 02 / Step 07 |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|
| W01 | source | benchmark/planning/seed-blinded-defects.sh | semantic manifest validation | private metadata | Validate semantic fields, final hashes, and private snapshot | — | 01-semantic-oracle-contract | 01-step-semantic-manifest |
| W02 | source | benchmark/planning/grade-blinded-run.sh | semantic adjudication | one-to-many finding coverage | Grade consolidated findings and report semantic plus diagnostic metrics | W03 | 01-semantic-oracle-contract | 03-step-adjudicated-coverage |
| W03 | source | benchmark/planning/review-oracle.sh | redacted adjudication envelope | candidate finding envelope | Define independent-oracle input/output and redaction boundary | W01 | 01-semantic-oracle-contract | 02-step-adjudication-envelope |
| W04 | source | benchmark/planning/setup-benchmark.sh | approval state | review/adoption state machine | Separate terminal review, approval, oracle completion, and adoptability | W03 | 02-approval-state-integrity | 01-step-approval-state-machine |
| W05 | docs | planning/SKILL.md | reviewer contract | Reviewer B approval evidence | Clarify false approval, evidence fields, and consolidated findings | W04 | 02-approval-state-integrity | 02-step-reviewer-contract |
| W06 | docs | benchmark/planning/analyzer-prompt.md | report schema | approval metrics | Require analyzer to preserve state truth table and thresholds | W04,W05 | 02-approval-state-integrity | 03-step-analyzer-report-schema |
| W07 | test | benchmark/planning/tests/test-review-oracle.sh | semantic oracle contract test | consolidated finding cases | Verify semantic manifest, adjudication, and exact-ID diagnostics | W01,W02,W03 | 03-regression-and-release-gates | 01-step-regression-oracle |
| W08 | test | benchmark/planning/tests/test-review-lifecycle.sh | approval state contract test | approval-conflict fixtures | Verify review completion versus adoption state | W04,W05 | 03-regression-and-release-gates | 02-step-regression-approval |
| W09 | test | benchmark/planning/tests/test-safeguards.sh | blinding boundary test | private/public redaction | Verify no keys, IDs, manifests, or private paths leak | W01,W03,W06 | 03-regression-and-release-gates | 03-step-regression-boundary |
| W10 | data | benchmark/planning/tests/fixtures/pilot-consolidated-finding.json | pilot failure fixture | AR-01 semantic coverage | Pin expected semantic coverage independently of historical archives | — | 03-regression-and-release-gates | 04-step-pilot-fixture |
| W11 | verification | N/A | current-protocol release gate | adoption-state output | Run worker, Reviewer B, oracle, analyzer, archive, and final validators | W07,W08,W09,W10 | 03-regression-and-release-gates | 05-step-end-to-end-gate |
| W12 | test | benchmark/planning/tests/test-review-oracle.sh | semantic manifest schema test | manifest validation | Verify required private semantic fields and final hashes | W01 | 01-semantic-oracle-contract | 04-step-manifest-schema-test |
| W13 | test | benchmark/planning/tests/test-review-lifecycle.sh | analyzer state schema test | report truth table | Verify explicit report fields and fail-closed reasons | W04,W06 | 02-approval-state-integrity | 04-step-analyzer-schema-test |
| W14 | generated | planning/REVIEWER.md | reviewer projection | approval evidence | Regenerate the reviewer projection from the contract | W05 | 02-approval-state-integrity | 05-step-reviewer-projection |
| W15 | docs | benchmark/planning/worker-prompt.md | worker reviewer prompt | protocol evidence | Align worker prompt with the reviewer contract | W05 | 02-approval-state-integrity | 06-step-worker-prompt |
| W16 | source | benchmark/planning/setup-benchmark.sh | protocol metadata | evaluation state | Publish explicit state schema and reasons | W04,W06 | 02-approval-state-integrity | 07-step-protocol-metadata |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
