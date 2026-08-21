# Work-unit inventory: reviewer-oracle-evidence-hardening

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|




| Goal-local envelope regression is exercised | W11 | The lossless contract goal has its own executable regression. |

| Goal-local authority and provenance regression is exercised | W12 | The authority goal has its own executable regression. |

| Complete reviewer finding envelopes survive approval serialization and semantic grading | W01,W02,W03,W04,W11 | A consolidated AR finding with path, location, summary, observed contradiction, impact, evidence, correction, and independence must produce three semantic true positives in the blinded oracle. |

| Only Reviewer B controls final approval and all provenance is attributable | W05,W06,W07 | Reviewer A handoffs cannot create approval conflicts, and archived evidence identifies the selected Reviewer B session, capsule, plan hash, seed snapshot hash, and transcript. |

| The full seeded benchmark path proves detection and fail-closed malformed evidence | W08,W09,W10,W14,W16 | Direct oracle, actual harness adapter, lifecycle state, injection seam, and malformed-envelope cases are covered by executable tests and a bounded benchmark run. |


| Setup adapter integration is exercised rather than bypassed | W14 | The test invokes the actual adapter path and inspects generated evidence/state/provenance artifacts. |

| Reviewer B approval schema and identity are validated before grading | W13,W15 | A malformed, stale, wrong-session, wrong-mode, or duplicate approval cannot become terminal evidence. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | source | `benchmark/planning/setup-benchmark.sh` | `approval-to-oracle evidence serialization block` | `N/A` | Preserve finding ID, repository-relative path, precise location, summary, observed contradiction, impact, evidence, required correction, and independence provenance in oracle-terminal-evidence.json. | — | 01-lossless-finding-contract | 01-step-preserve-finding-envelope |


| W03 | source | `benchmark/planning/setup-benchmark.sh` | `Reviewer prompt construction` | `N/A` | Tell Reviewer B exactly which finding fields are mandatory and reject ID-only or narrative-only approval evidence before oracle grading. | W02 | 01-lossless-finding-contract | 03-step-specify-reviewer-schema |

| W04 | docs | `benchmark/planning/worker-prompt.md` | `final Reviewer B handoff contract` | `N/A` | Make the worker-facing protocol require complete machine-readable AR-NN finding objects with precise path/location, contradiction, impact, evidence, and correction. | W03 | 01-lossless-finding-contract | 04-step-document-finding-contract |

| W05 | source | `benchmark/planning/setup-benchmark.sh` | `reviewer approval state calculation` | `N/A` | Use exactly one authoritative Reviewer B terminal approval, reject Reviewer A overall approvals as protocol violations, and distinguish missing, duplicate, conflicting, and rejected final approvals. | W01 | 02-authority-and-provenance | 01-step-separate-reviewer-authority |



| W08 | test | `benchmark/planning/tests/test-review-oracle.sh` | `blinded semantic oracle integration fixture` | `N/A` | Extend the fixture to prove one complete consolidated AR finding catches all three seeded defects and that the published report remains redacted. | W02 | 03-end-to-end-proof | 01-step-test-consolidated-finding |


| W10 | verification | `N/A` | `bounded seeded benchmark and completion gate` | `N/A` | Run focused contracts plus one iterative and one fresh current-protocol control, then verify 3/3 consolidated semantic and independent catches or fail closed with an explicit unavailable reason. | W08,W09,W14,W16 | 03-end-to-end-proof | 03-step-run-end-to-end-gate |

| W11 | test | `benchmark/planning/tests/test-review-oracle.sh` | `complete finding envelope fixture` | `N/A` | Provide a goal-local regression test for the exact fields W01 and W02 transport and grade. | W01,W02 | 01-lossless-finding-contract | 05-step-contract-regression |

| W12 | test | `benchmark/planning/tests/test-review-lifecycle.sh` | `Reviewer B authority fixture` | `N/A` | Provide a goal-local regression test for sole Reviewer B authority and provenance requirements. | W05,W06 | 02-authority-and-provenance | 04-step-authority-regression |

| W02 | source | `benchmark/planning/grade-blinded-run.sh` | `valid_envelope()` | `N/A` | Require all envelope fields; emit schema_status malformed, per-finding reason codes, malformed_count, REVIEW_FINDING_SCHEMA_INVALID, and adoption false for invalid input; retain consolidated-finding matching across multiple seeded defects. | W01 | 01-lossless-finding-contract | 02-step-validate-finding-envelope |

| W06 | source | `benchmark/planning/setup-benchmark.sh` | `protocol-metadata publication block` | `N/A` | Record selected Reviewer B session/capsule, source and defective plan hashes, target snapshot, approval, transcript, lifecycle handoff hashes, and cross-checkable archive paths. | W05,W13,W15 | 02-authority-and-provenance | 02-step-record-provenance |

| W07 | docs | `benchmark/planning/analyzer-prompt.md` | `reviewer lifecycle interpretation section` | `N/A` | Define A as handoff-only, B as final authority, and require analysis to report schema failures, provenance, and adapter transformations without inferring success. | W06 | 02-authority-and-provenance | 03-step-update-analysis-contract |

| W09 | test | `benchmark/planning/tests/test-review-lifecycle.sh` | `approval adapter assertion group` | `N/A` | Test lossless field preservation, malformed-envelope rejection with reason codes/count/state, Reviewer A authority prohibition, sole Reviewer B selection, and explicit state reasons. | W05,W06,W15 | 03-end-to-end-proof | 02-step-test-lifecycle-adapter |


| W14 | test | `benchmark/planning/tests/test-review-lifecycle.sh` | `setup-benchmark integration fixture` | `N/A` | Invoke the actual setup adapter through W16’s deterministic command seam with fixed fixture values and assert every evidence/state/provenance field and malformed result. | W01,W05,W06,W15,W16 | 03-end-to-end-proof | 04-step-test-setup-integration |

| W16 | source | `benchmark/planning/setup-benchmark.sh` | `reviewer_command_injection` | `N/A` | Add test-only command and deterministic session/capsule/mode/timestamp inputs for W14 while preserving production defaults and all validation gates. | W05,W13 | 03-end-to-end-proof | 05-step-add-adapter-injection-seam |

| W13 | source | `benchmark/planning/setup-benchmark.sh` | `approval_schema_validator()` | `N/A` | Validate approval.json shape, required approval metadata, and complete finding object types before serialization. | W05 | 02-authority-and-provenance | 05-step-validate-approval-identity |

| W15 | source | `benchmark/planning/setup-benchmark.sh` | `reviewer_b_session_binding` | `N/A` | Bind the selected approval to the exact Reviewer B lifecycle session and capsule; reject wrong-session, wrong-mode, stale, duplicate, and mismatched artifacts before terminal evidence. | W05,W13 | 02-authority-and-provenance | 06-step-bind-reviewer-session |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
