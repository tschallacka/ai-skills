# Work-unit inventory: reviewer-adjudicator-hardening

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| Tolerant path/location matching (multi-file path segments, prose/section locations) | W01,W02 | make schema-valid findings gradeable per RC-01/RC-02 |

| Reviewer evidence contract documented so schema-valid equals gradeable | W03 | contract note in prompt and schema validator |




| Frozen-archive regrade expectations locked and suites passing | W12,W13,W14,W15 | iterative 3/3, fresh 2/3; no archive edits |

| Mutated conflict token available to the grader (manifest carries old/new) | W16 | enable the W05 mutated-conflict requirement and frozen replay |

| Symmetric robust semantic matching (normalization, signal token-overlap, correction parity, mutated-conflict) | W04,W05,W06 | RC-03 fix; W05 depends on W16 mutated token |

| Thresholds recorded and denominator vs threshold reasons distinct | W07,W08 | RC-05 fix |

| Per-defect explainable classification, sanitized public projection, reproducible report | W09,W10,W11 | RC-04 diagnosability; analysis recommendation 1 |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | source | `benchmark/planning/grade-blinded-run.sh` | `candidate_path + path filter` | `N/A` | Accept a defect file if it appears as any ; separated path segment or is resolvable from location or evidence; preserve exact single-file equality. | — | 01-grading-contract-alignment | 01-step-tolerant-path |

| W02 | source | `benchmark/planning/grade-blinded-run.sh` | `location_matches` | `N/A` | Normalize section markers (section/sec/3.1/#/:) and accept line- or prose-location citations that reference the defect file and section; keep exact S-location equality passing. | W01 | 01-grading-contract-alignment | 02-step-location-normalization |

| W03 | docs | `benchmark/planning/setup-benchmark.sh` | `reviewer prompt + approval schema contract note` | `N/A` | Document the reviewer-evidence contract (single-file path recommended when a defect is in one file, file section location recommended, prose accepted, consolidation valid) in the generated reviewer prompt and schema validator comments so schema-valid equals gradeable. | W01 | 01-grading-contract-alignment | 03-step-evidence-contract-note |

| W04 | source | `benchmark/planning/grade-blinded-run.sh` | `normalize tokens/ordinals/hyphens` | `N/A` | Add a shared normalizer that lowercases, strips hyphens and non-alphanumerics for word compare, and maps ordinal/digit forms (4/fourth, 3/third) so fourth generated button equals fourth-generated-button and generated button 4. | W01 | 02-semantic-matcher-robustness | 01-step-normalization-helper |


| W06 | source | `benchmark/planning/grade-blinded-run.sh` | `correction_matches parity` | `N/A` | Review correction_matches against the new normalizer and mutated-conflict rule for parity and add regression notes for the 50 percent token-overlap fallback on short expected corrections. | W05 | 02-semantic-matcher-robustness | 03-step-correction-parity |




| W10 | config | `benchmark/planning/setup-benchmark.sh` | `per-defect aliases in metadata/telemetry` | `N/A` | Surface a public projection of per_defect diagnostics into protocol-metadata and telemetry aliases without leaking expected_signal, required_correction, or seed IDs. | W09 | 04-per-defect-diagnostics | 02-step-alias-expose |

| W11 | verification | `N/A` | `N/A` | `N/A` | Produce and archive a deterministic post-run report showing each defect, the candidate findings considered, and the exact failed predicate; verify it is reproducible across two invocations. | W10 | 04-per-defect-diagnostics | 03-step-post-run-report |

| W12 | test | `benchmark/planning/tests/test-review-oracle.sh` | `fixtures for multi-path/prose/hyphen/paraphrase/section` | `N/A` | Extend oracle fixtures to cover multi-file path, prose location, section/sec/S variants, hyphenated signal, paraphrase signal, and mutated-conflict positive and negative cases. | W09 | 05-regression-fixtures-and-replay | 01-step-fixture-expansion |

| W13 | test | `benchmark/planning/tests/test-review-lifecycle.sh` | `threshold reason split case` | `N/A` | Add lifecycle cases asserting MISSING_THRESHOLDS when thresholds are absent and that MISSING_DENOMINATOR does not fire when the oracle reports a valid denominator. | W08 | 05-regression-fixtures-and-replay | 02-step-threshold-lifecycle-test |

| W14 | test | `benchmark/planning/tests/test-frozen-replay.sh` | `frozen archive regrade expectations` | `N/A` | Add a deterministic test that regrades the frozen approval.json archives against pilot-blinded-defects.json and pins iterative 3/3 and fresh 2/3 as expectations without editing archives. | W12 | 05-regression-fixtures-and-replay | 03-step-frozen-replay |

| W15 | verification | `N/A` | `N/A` | `N/A` | Run test-review-oracle.sh, test-review-lifecycle.sh, test-frozen-replay.sh, the plan validator, and the oracle/lifecycle suites; all must pass. | W14 | 05-regression-fixtures-and-replay | 04-step-full-suite |

| W07 | source | `benchmark/planning/run-benchmark.sh` | `threshold export before setup` | `N/A` | Export SEMANTIC_THRESHOLD and INDEPENDENT_THRESHOLD (from config or passed values) before invoking setup-benchmark.sh so the state synthesizer receives real values instead of empty strings. | — | 03-state-threshold-fix | 01-step-export-thresholds |

| W08 | source | `benchmark/planning/setup-benchmark.sh` | `reviewer-state threshold/denominator reasons` | `N/A` | Split MISSING_DENOMINATOR from MISSING_THRESHOLDS: MISSING_DENOMINATOR fires only when the oracle denominator is absent/invalid/zero; MISSING_THRESHOLDS fires only when threshold values are absent; record real thresholds in state and metadata. | W07 | 03-state-threshold-fix | 02-step-split-denominator-reasons |

| W09 | source | `benchmark/planning/grade-blinded-run.sh` | `per-defect classification` | `report schema` | Emit per_defect entries with defect_id, classification (true_positive/partial/unresolved/false_positive), candidate finding ids, and the list of failed predicates, while keeping aggregate fields and public redaction. | W05 | 04-per-defect-diagnostics | 01-step-per-defect-report |

| W16 | source | `benchmark/planning/seed-blinded-defects.sh` | `defect-map manifest old/new fields` | `N/A` | Persist the mutated old and new tokens into each defect-map manifest entry so the grader can require the finding to reference the mutated conflict; keep expected_signal and required_correction unchanged. Frozen replay sources old/new from pilot-blinded-defects.json. | — | 02-semantic-matcher-robustness | 04-step-manifest-mutated-token |

| W05 | source | `benchmark/planning/grade-blinded-run.sh` | `signal_matches mutation-conflict + token-overlap` | `N/A` | Give signal matching a token-overlap fallback symmetric with correction, and require a true positive to reference the mutated conflict using the defect old/new tokens now carried by the manifest (W16), falling back to an explicit inconsistency indicator only when a mutation token is absent; minimum token-overlap floor for short signals. | W04,W16 | 02-semantic-matcher-robustness | 02-step-signal-overlap |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
