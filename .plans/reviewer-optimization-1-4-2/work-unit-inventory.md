# Work-unit inventory: reviewer-optimization-1-4-2

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| Authoritative 1.4.2 contract and generated reviewer profile | W01,W02,W03,W04,W05 | Covers the source contract, generated artifact, and generation proof. |

| Bounded iterative review lifecycle | W06,W07,W08,W09,W10 | Covers runner state, worker/analyzer contracts, benchmark rules, and lifecycle tests. |

| Capsule access boundary | W11,W12,W13,W14,W15 | Covers capsule construction, worker/analyzer restrictions, and escape tests. |

| Bounded context and checkpoints | W16,W17,W18,W19,W20 | Covers source-aware context indexing, bounded views, wrappers, checkpoints, and tests. |

| Cohort transition and archive provenance | W21,W22,W23,W24,W25,W26,W27 | Covers protocol metadata, atomic archives, analyzer merge rules, operator docs, and package ownership. |

| Safeguards and evidence integrity | W28,W29,W30,W31,W32,W33 | Covers cancellation, telemetry, taint layers, raw evidence, journeys, and safeguard tests. |

| Pilot and release gate | W34,W35,W36,W37,W38 | Covers contract/package tests, pilot execution, decision analysis, and final validation. |

| Pilot execution hygiene | W62 | Covers reusable temporary helpers for long repeated monitoring commands and their bounded cleanup verification. |

| Final implementation comment cleanup | W63 | Covers the post-evaluation review that removes journey comments and keeps only concise comments required to explain non-obvious behavior. |

| Dynamic plan mutation and persistent execution | W64,W65,W66 | Covers fully validated plan additions, monitor guidance that continues active subprocesses through intermediate status reports, and focused fixture tests. |

| Planning environment manifests | W67,W68,W69 | Covers global and plan-local `.env` creation, safe helper consumption, refresh/permission behavior, and full fixture validation. |

| Environment contract evolution | W70,W71 | Covers the no-backward-compatibility protocol for future manifest variable/schema changes and fail-closed stale/unknown schema tests. |

| Reviewer projection synchronization | W72,W73 | Covers regeneration and hash/section validation of the generated reviewer projection. |

| Archive manifest exclusion | W74,W75 | Covers excluding local `.env` manifests from published archives while retaining ordinary evidence. |

| Manifest path security | W76,W77 | Covers complete path containment, ownership, duplicate-assignment, and shell-expansion rejection. |

| Manifest consumer interface | W78,W79 | Covers aligning the documented and implemented validated manifest-loading interface. |

| No legacy fallback | W80,W81 | Covers removal or strict isolation of the benchmark context-fixture fallback and regression tests. |

| Package regression coverage | W82,W83 | Covers packaging and installer verification of the new safety regression files. |

| Final code-to-plan review | W84,W85 | Covers the final implementation comparison and explicit user decision on amendments or accepted exceptions. |

| Blinded seeded-defect oracle | W86,W87,W88,W89 | Covers isolated encrypted defect seeding, target/key separation, independent oracle grading, and genuine retained classifications. |

| Outer | W39 | Added after independent review; owns the newly identified proof boundary. |

| Reviewer | W40 | Added after independent review; owns the newly identified proof boundary. |

| Review | W41 | Added after independent review; owns the newly identified proof boundary. |

| Reviewer | W42 | Added after independent review; owns the newly identified proof boundary. |

| Capsule | W43 | Added after independent review; owns the newly identified proof boundary. |

| Worker | W44 | Added after independent review; owns the newly identified proof boundary. |

| Analyzer | W45 | Added after independent review; owns the newly identified proof boundary. |

| Capsule | W46 | Added after independent review; owns the newly identified proof boundary. |

| Checkpoint | W47 | Added after independent review; owns the newly identified proof boundary. |

| Helper | W48 | Added after independent review; owns the newly identified proof boundary. |

| Context | W49 | Added after independent review; owns the newly identified proof boundary. |

| Atomic | W50 | Added after independent review; owns the newly identified proof boundary. |

| Archive | W51 | Added after independent review; owns the newly identified proof boundary. |

| Telemetry | W52 | Added after independent review; owns the newly identified proof boundary. |

| Telemetry | W53 | Added after independent review; owns the newly identified proof boundary. |

| Telemetry | W54 | Added after independent review; owns the newly identified proof boundary. |

| Fresh | W55 | Added after independent review; owns the newly identified proof boundary. |

| Control | W56 | Added after independent review; owns the newly identified proof boundary. |

| Independent | W57 | Added after independent review; owns the newly identified proof boundary. |

| Seeded defect oracle and matched pilot control | W58,W59,W60 | Closes baseline accuracy, threshold, and phase telemetry proof. |

| Executable benchmark mode and parser contract | W61 | Owns the command syntax used by the pilot and control verification units. |

| Helper-only durable plan mutation governance | W90,W91,W92 | Covers helper-dispatched plan creation, testing-companion/progress support, bulk execution guidance, and regression validation. |

| Helper-only durable plan mutation governance | W93,W94,W95,W96 | Goal 21 definition-of-done coverage for companion creation, aggregate progress rebuilding, adversarial-finding insertion, and complete regression verification. |

| Plan helper lifecycle remains valid after helper additions | W97,W99 | Goal 21 coverage for add-goal progress bootstrap and its regression test. |

| Independent blinded grader is implemented and tested | W98,W100 | Goal 20 coverage for standalone grading and its protocol regression. |


## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | source | `planning/SKILL.md` | `iterative review contract` | `N/A` | Define authoritative iterative-review mode, reviewer lifecycle fields, bounded pass/cycle limits, contamination rules, access-control boundary, protocol metadata, telemetry schema, and legacy-cohort transition. | — | 01-contract-profile | 01-step-skill-contract |

| W02 | source | `planning/scripts/generate-reviewer.sh` | `main profile-generation flow` | `N/A` | Enforce REVIEWER_SECTION allowlisting, required-section validation, source hashing, version 1.4.2 metadata, and non-hand-edited output generation. | W01 | 01-contract-profile | 02-step-reviewer-generator |

| W03 | generated | `planning/REVIEWER.md` | `generated reviewer profile` | `N/A` | Regenerate the reviewer profile from the tagged source sections and record the source hash and version without hand edits. | W02 | 01-contract-profile | 03-step-reviewer-profile |

| W04 | test | `planning/tests/test-plan-commands.sh` | `reviewer profile contract tests` | `N/A` | Test marked-section extraction, missing/empty section rejection, hash/version metadata, and generated-profile drift detection. | W01,W02,W03 | 01-contract-profile | 04-step-profile-tests |

| W05 | verification | `N/A` | `reviewer-profile generation and contract test command` | `N/A` | Run generator and the focused planning contract tests against a clean temporary copy and prove the committed profile matches generated output. | W04 | 01-contract-profile | 05-step-profile-verification |

| W06 | source | `benchmark/planning/run-benchmark.sh` | `review lifecycle state declarations` | `N/A` | Define bounded review-cycle state, reviewer session records, pass/cycle limits, and lifecycle event fields; process launch, handoff, and signal cleanup are owned by W40/W41/W39. | W01 | 02-review-lifecycle | 01-step-runner-lifecycle |

| W07 | docs | `benchmark/planning/worker-prompt.md` | `iterative-review-mode instructions` | `N/A` | Specify Reviewer A finding ownership, bounded verification passes, concise handoff/termination, and fresh reviewer replacement rules while preserving fresh-review default behavior. | W01 | 02-review-lifecycle | 02-step-worker-review-contract |

| W08 | docs | `benchmark/planning/analyzer-prompt.md` | `review lifecycle report` | `N/A` | Require separate reporting of review cycles, verification passes, termination/handoff events, independence status, and unresolved limits. | W07 | 02-review-lifecycle | 03-step-analyzer-review-report |

| W09 | docs | `benchmark/planning/benchmark-test.md` | `review protocol requirements` | `N/A` | Document iterative mode inputs, hard limits, default-mode compatibility, and acceptance criteria for Reviewer B independent defect detection. | W06,W07 | 02-review-lifecycle | 04-step-benchmark-contract |

| W10 | test | `benchmark/planning/tests/test-review-runner.sh` | `review-cycle option test` | `N/A` | Exercise option validation, fresh-review fallback, maximum-pass termination, and lifecycle metadata in a bounded harness fixture. | W06,W09 | 02-review-lifecycle | 05-step-runner-tests |

| W11 | source | `benchmark/planning/setup-benchmark.sh` | `render_template()` | `N/A` | Build a clean per-worker capsule containing only task inputs, tagged planning skill, REVIEWER.md for reviewers, and resolved relative references; keep capsule, workspace, and archive roots separate. | W01 | 03-capsule-access | 01-step-capsule-builder |

| W12 | source | `benchmark/planning/setup-benchmark.sh` | `generated start-worker.sh access boundary` | `N/A` | Launch workers with capsule-only filesystem scope, create per-worker .bm-vars and wrapper paths, and record command/path access attempts for tainting. | W11 | 03-capsule-access | 02-step-worker-boundary |

| W13 | docs | `benchmark/planning/worker-prompt.md` | `filesystem allowlist` | `N/A` | Constrain worker reads to the capsule and workspace, forbid repository history/installed skills/previous results, and require explicit escape reporting. | W12 | 03-capsule-access | 03-step-worker-access-prompt |

| W14 | docs | `benchmark/planning/analyzer-prompt.md` | `analyzer filesystem allowlist` | `N/A` | Constrain analyzers to the run instructions, summary, and current result archive while preserving access-audit evidence and taint semantics. | W12 | 03-capsule-access | 04-step-analyzer-access-prompt |

| W15 | test | `benchmark/planning/tests/test-capsule-access.sh` | `capsule access test functions` | `N/A` | Verify allowed files are readable, unallowlisted source and prior result roots are unavailable, and escape attempts create tainted evidence. | W11,W12,W13,W14 | 03-capsule-access | 05-step-access-tests |

| W16 | source | `planning/scripts/plan-context-lib.sh` | `context_build_index()` | `N/A` | Add source namespaces for SKILL.md, REVIEWER.md, and approved relative references, retain hash freshness checks, and invalidate memory when sources or plan files change. | W01 | 04-context-checkpoints | 01-step-context-index |

| W17 | source | `planning/scripts/plan-context.sh` | `context read/check command dispatch` | `N/A` | Expose bounded phase-specific summary, ownership/dependency, changed-document, and validator-focused views with fixed command contracts and size limits. | W16 | 04-context-checkpoints | 02-step-context-views |

| W18 | source | `benchmark/planning/setup-benchmark.sh` | `benchmark-env.sh generation` | `N/A` | Generate isolated per-worker variables and wrapper paths before invoking plan-context.sh, avoiding shared mutable variable files. | W12,W17 | 04-context-checkpoints | 03-step-context-wrapper |

| W19 | docs | `benchmark/planning/worker-prompt.md` | `phase checkpoint protocol` | `N/A` | Require compact checkpoints after drafting, review, correction, and validation with hashes, changed files, open findings, and next action. | W18 | 04-context-checkpoints | 04-step-checkpoint-contract |

| W20 | test | `planning/tests/test-plan-context.sh` | `context cache test function` | `N/A` | Verify bounded reads, changed-entry detection, source namespace freshness, invalidation after mutation, and no shared-state collision. | W16,W17 | 04-context-checkpoints | 05-step-context-tests |

| W21 | source | `benchmark/planning/run-benchmark.sh` | `select_latest_tags()` | `N/A` | Start protocol 1.4.2 as a distinct cohort, preserve legacy results unchanged, and record protocol, reviewer mode, and access-control mode in run metadata. | W06 | 05-protocol-archive | 01-step-protocol-cohort |

| W22 | source | `benchmark/planning/setup-benchmark.sh` | `evaluation template metadata` | `N/A` | Define the self-describing protocol, cohort, skill/version, reviewer mode, access mode, schema version, and selected-plan metadata consumed by publication owner W50. | W11,W21 | 05-protocol-archive | 02-step-atomic-archive |

| W23 | docs | `benchmark/planning/analyzer-prompt.md` | `legacy/new cohort merge report` | `N/A` | Merge legacy and 1.4.2 results into one report with separate cohort summaries, protocol labels, contextual cross-cohort comparisons, and authoritative within-cohort metrics. | W08,W21 | 05-protocol-archive | 03-step-cohort-analyzer |

| W24 | docs | `benchmark/planning/benchmark-test.md` | `protocol transition` | `N/A` | Document frozen legacy treatment, 1.4.2 start boundary, metadata requirements, unchanged historical layout, and analyzer comparison rules. | W21,W22,W23 | 05-protocol-archive | 04-step-cohort-contract |

| W25 | docs | `benchmark/planning/README.md` | `benchmark operator workflow` | `N/A` | Document the user-facing setup command, hidden protocol metadata, capsule lifecycle, pilot command, and result archive locations without exposing implementation differences. | W21,W22 | 05-protocol-archive | 05-step-readme |

| W26 | config | `planning/V27-PACKAGE-MANIFEST.txt` | `1.4.2 installable package file list` | `N/A` | Extend the finite manifest for the v27 replacement package to include the changed contract, helpers, benchmark/oracle records, fixtures, and runner evidence. | W01,W16,W21 | 05-protocol-archive | 06-step-package-manifest |

| W27 | config | `planning/V27-PACKAGE-MAP.tsv` | `source/destination ownership map` | `N/A` | Map every new or changed package file to its install destination and preserve the explicit six-column ownership boundary. | W26 | 05-protocol-archive | 07-step-package-map |

| W28 | source | `benchmark/planning/setup-benchmark.sh` | `generated worker cleanup_on_signal()` | `N/A` | Forward an interrupt to the worker-owned process group and descendants, wait for cleanup, remove case temporary state, and preserve interrupted evidence. Outer batch propagation is owned by W39. | W12,W39 | 06-safeguards-telemetry | 01-step-process-cancellation |

| W29 | source | `benchmark/planning/telemetry.sh` | `database discovery` | `N/A` | Resolve configured/current telemetry stores system-independently, match exact worker UUIDs, reject stale or ambiguous matches, and record database path and lookup method. | W12 | 06-safeguards-telemetry | 02-step-telemetry-integrity |

| W30 | source | `benchmark/planning/setup-benchmark.sh` | `STATUS calculation` | `N/A` | Consume W60 final telemetry.json, synthesize evaluation/status and taint causes from audit evidence, and preserve raw evidence without treating raw or rejected telemetry as valid. | W28,W29,W60 | 06-safeguards-telemetry | 03-step-taint-layers |

| W31 | source | `benchmark/planning/telemetry.sh` | `ROLLOUT_FILE output path` | `N/A` | Define the retained raw telemetry artifact path and archive handoff consumed by W53/W60, without duplicating extraction or schema validation. | W29 | 06-safeguards-telemetry | 04-step-telemetry-artifact |

| W32 | docs | `benchmark/planning/analyzer-prompt.md` | `developer journey evidence rules` | `N/A` | Require concise per-version journeys covering review rounds, findings, fixes, validation attempts, artifact expansion, and latency/token deltas; label missing evidence unavailable. | W08,W23,W30,W31 | 06-safeguards-telemetry | 05-step-developer-journey |

| W33 | test | `benchmark/planning/tests/test-safeguards.sh` | `process test function` | `N/A` | Prove no worker/reviewer/child survives interruption and combined process/access/worker/reviewer/analyzer failures retain separate taint causes. | W28,W30 | 06-safeguards-telemetry | 06-step-safeguard-tests |

| W34 | test | `planning/tests/test-planning-context-v27-contract.sh` | `1.4.2 context contract matrix` | `N/A` | Extend the v27 oracle/benchmark contract tests for capsule variables, source namespaces, checkpoint invalidation, bounded retry, and compact-read behavior. | W16,W17,W18 | 07-pilot-release | 01-step-context-contract-tests |

| W35 | test | `planning/tests/test-installer-manifest.sh` | `v27 manifest test function` | `N/A` | Verify exact manifest coverage, destination mapping, collision/approval failure behavior, and no partial install for the 1.4.2 package. | W26,W27 | 07-pilot-release | 02-step-installer-tests |

| W36 | verification | `N/A` | `one-to-two revision pilot command` | `N/A` | Run the bounded 1.4.2 pilot against one or two revisions with iterative mode and mandatory fresh final review, retaining complete archives and telemetry. | W10,W15,W20,W33,W34,W35 | 07-pilot-release | 03-step-pilot-run |

| W37 | verification | `N/A` | `pilot comparison and decision rule` | `N/A` | Compare tokens, reviewer events, findings, fixes, final validation, taint rate, independent defect detection, and archive completeness against fresh-review mode; adopt only if the decision rule passes. | W23,W32,W36 | 07-pilot-release | 04-step-pilot-analysis |

| W38 | verification | `N/A` | `full package and plan readiness validation` | `N/A` | Run the complete helper, manifest, contract-test, protocol, final-independent-review, oracle, archive, telemetry, phase-metric, and final implementation-comment cleanup validation suite before release. | W34,W35,W36,W37,W50,W51,W52,W53,W54,W57,W58,W59,W60,W63 | 07-pilot-release | 05-step-release-gate |

| W39 | source | `benchmark/planning/setup-and-run.sh` | `top-level signal trap` | `N/A` | Forward Ctrl+C/TERM to every active worker, reviewer, analyzer, and child process; wait for cleanup, remove temporary state, and return a distinct interrupted status. | W06 | 02-review-lifecycle | 06-step-outer-cancellation |

| W40 | source | `benchmark/planning/run-benchmark.sh` | `reviewer launch block` | `N/A` | Own reviewer session/cycle state, launch fresh reviewer sessions, enforce pass/cycle limits, and persist lifecycle records separately from worker/analyzer execution. | W06,W39 | 02-review-lifecycle | 07-step-reviewer-launcher |

| W41 | source | `benchmark/planning/run-benchmark.sh` | `review finding handoff artifacts` | `N/A` | Write changed-file/diff/targeted-validation handoffs, stable AR-NN ownership, closure passes, termination events, and final fresh-review approval artifacts. | W40 | 02-review-lifecycle | 08-step-review-handoffs |

| W42 | test | `benchmark/planning/tests/test-review-lifecycle.sh` | `review lifecycle test functions` | `N/A` | Test option validation, Reviewer A ownership limits, fresh Reviewer B isolation, handoff artifacts, final approval prohibition, and interruption propagation. | W39,W40,W41 | 02-review-lifecycle | 09-step-review-tests |

| W43 | discovery | `benchmark/planning/setup-benchmark.sh` | `approved relative-reference resolver` | `N/A` | Enumerate every relative reference required by SKILL.md/REVIEWER.md and determine the minimal capsule file set before implementing the capsule copy boundary. | W11 | 03-capsule-access | 06-step-capsule-discovery |

| W44 | source | `benchmark/planning/setup-benchmark.sh` | `generated start-worker.sh codex launch` | `N/A` | Replace full tagged-source --add-dir access with the worker capsule and explicitly expose only the workspace plus approved capsule paths; record command/path audits. | W43 | 03-capsule-access | 07-step-worker-launch-boundary |

| W45 | source | `benchmark/planning/run-benchmark.sh` | `analyzer launch block` | `N/A` | Create a fresh analyzer capsule containing only benchmark instructions, summary, and current run results, with no source checkout or previous result roots. | W43,W44 | 03-capsule-access | 08-step-analyzer-launch-boundary |

| W46 | test | `benchmark/planning/tests/test-capsule-access.sh` | `capsule access test functions` | `N/A` | Test physical availability of allowed files and denial/taint for broad source, installed skills, parent paths, previous results, and unapproved validator scripts. | W43,W44,W45 | 03-capsule-access | 09-step-capsule-tests |

| W47 | source | `planning/scripts/plan-context-lib.sh` | `checkpoint persistence` | `N/A` | Persist compact phase checkpoints outside counted plan deliverables, with current state, open findings, next action, changed files, hashes, and source/plan invalidation. | W16,W19 | 04-context-checkpoints | 06-step-checkpoint-storage |

| W48 | source | `planning/scripts/plan-document-lib.sh` | `plan helper output mode` | `N/A` | Add quiet-by-default helper output, explicit verbose mode, size budgets for reports/companions/context, and bounded malformed-call retry messages. | W17 | 04-context-checkpoints | 07-step-helper-budgets |

| W49 | test | `planning/tests/test-plan-context.sh` | `checkpoint test function` | `N/A` | Test checkpoint lifecycle, memory exclusion from counted deliverables, output budgets, quiet mode, and bounded retry behavior. | W47,W48 | 04-context-checkpoints | 08-step-checkpoint-budget-tests |

| W50 | source | `benchmark/planning/setup-benchmark.sh` | `result publication block` | `N/A` | Stage evaluation and copied artifacts under a private run directory, validate all preconditions, atomically rename on success, and clean up on failure/collision/interruption. | W22,W30,W31,W53,W60 | 05-protocol-archive | 08-step-atomic-publication |

| W51 | test | `benchmark/planning/tests/test-archive-integrity.sh` | `archive publication test functions` | `N/A` | Test worker/reviewer/analyzer/telemetry failures, collision behavior, rollback cleanup, atomic visibility, and exact tagged-skill provenance. | W50 | 05-protocol-archive | 09-step-archive-tests |

| W52 | config | `benchmark/planning/telemetry-schema.json` | `raw telemetry schema` | `N/A` | Define the validated machine-readable schema for worker/reviewer IDs, provenance, phase boundaries, token/cache composition, counts, durations, tool volumes, validator/patch/function-call metrics, command activity, and parent/child lifecycle. | W31 | 06-safeguards-telemetry | 07-step-telemetry-schema |

| W53 | source | `benchmark/planning/telemetry.sh` | `python extraction block` | `N/A` | Extract exact fields into raw.jsonl, mark heuristic/unavailable fields, preserve provenance, and reject malformed/stale/ambiguous source matches; final schema validation is owned by W60. | W29,W31 | 06-safeguards-telemetry | 08-step-telemetry-extraction |

| W54 | test | `benchmark/planning/tests/test-telemetry-integrity.sh` | `telemetry extraction test functions` | `N/A` | Test complete, partial, malformed, missing, stale, ambiguous, exact, heuristic, and rollout-fallback telemetry fixtures with provenance. | W52,W53 | 06-safeguards-telemetry | 09-step-telemetry-tests |

| W55 | verification | `N/A` | `matched fresh-review control run` | `N/A` | Run the same one-to-two revision matrix with default fresh-review mode, matching task, environment, artifact checks, and telemetry requirements. | W09,W36 | 07-pilot-release | 06-step-fresh-control |

| W56 | verification | `N/A` | `iterative-vs-control metric analysis` | `N/A` | Compute token and latency deltas, reviewer event/finding/fix counts, final validation, taint rate, and independent defect detection; fail adoption when evidence is unavailable or the decision rule is not met. | W37,W55 | 07-pilot-release | 07-step-control-analysis |

| W57 | verification | `N/A` | `final independent review gate` | `N/A` | Require a fresh final reviewer approval, no open AR findings, complete lifecycle handoff records, and preserved Reviewer B session evidence before release validation passes. | W41,W56 | 07-pilot-release | 08-step-independent-gate |

| W58 | test | `benchmark/planning/tests/test-review-oracle.sh` | `seeded defect oracle test function` | `N/A` | Define a blinded seeded-defect set and calculate true positives, false negatives, independent catches, duplicates, unresolved findings, and accuracy denominators for iterative and fresh-review runs. | W42,W55 | 07-pilot-release | 09-step-review-oracle |

| W59 | verification | `N/A` | `fixed pilot matrix and acceptance thresholds` | `N/A` | Run exactly one iterative and one fresh-review control per selected revision using fixed tags, task inputs, isolated roots, protocol metadata, and fail-closed token/latency/defect-detection thresholds. | W36,W55,W56 | 07-pilot-release | 10-step-pilot-thresholds |

| W60 | source | `benchmark/planning/telemetry.sh` | `phase metrics extraction block` | `N/A` | Augment W53 raw telemetry, validate the complete W52 schema, and write final telemetry.json or telemetry-rejection.json with phase durations, command/read/output/retry counts, artifact sizes, reviewer durations/tokens, and exact-versus-heuristic status. | W52,W53 | 06-safeguards-telemetry | 10-step-phase-metrics |

| W61 | source | `benchmark/planning/setup-and-run.sh` | `argument parser` | `N/A` | Accept the explicit --iterative/--fresh-review mode, --revisions tag list, run name, scheduling mode, and per-worker limits while rejecting malformed combinations before any process starts. | W39 | 02-review-lifecycle | 10-step-runner-arguments |

| W62 | verification | `N/A` | `reusable monitoring command helper` | `N/A` | When a long command is executed repeatedly during benchmark monitoring or validation, write it once as an executable helper under `/tmp` with safe argument handling, bounded output, and documented cleanup; invoke the helper thereafter instead of repeating the full command text. | W59,W61 | 08-command-execution-hygiene | 01-step-command-helper-reuse |

| W63 | verification | `N/A` | `final implementation comment cleanup` | `N/A` | After evaluation and implementation are complete, review developer-journey comments in changed code; remove comments made unnecessary by self-documenting code and refactor remaining comments into concise explanations of non-obvious intent, constraints, or trade-offs. | W62 | 09-implementation-comment-cleanup | 01-step-comment-cleanup |

| W64 | source | `planning/SKILL.md` | `validated dynamic plan mutation contract` | `N/A` | Require every newly discovered implementation scope to become a linked goal/step/work-unit/testing/progress/dependency/evidence update, followed by validator and adversarial-review checks; reject note-only additions as incomplete plan state. | W48,W61 | 10-plan-integrity-and-execution-persistence | 01-step-dynamic-plan-mutation |

| W65 | docs | `benchmark/planning/README.md` | `monitor continuation contract` | `N/A` | Define stringent monitor behavior for active subprocesses: treat status reports as intermediate, poll bounded process/output state, steer continuation with explicit next actions, stop only on terminal evidence or recorded blocker, and preserve evidence on interruption. | W39,W61 | 10-plan-integrity-and-execution-persistence | 02-step-monitor-continuation |

| W66 | test | `planning/tests/test-plan-integrity-and-monitor.sh` | `integrity monitor fixture tests` | `N/A` | Test note-only versus complete plan mutations, status-only subprocess output, bounded steering, retry exhaustion, terminal classification, and interruption evidence. | W64,W65 | 10-plan-integrity-and-execution-persistence | 03-step-integrity-monitor-tests |

| W67 | source | `planning/scripts/create-plan.sh` | `environment manifest creation` | `N/A` | Create or refresh `~/.plans/.env` and `.plans/<plan-name>/.env` with stable quoted path variables, deterministic plan-local paths, restrictive permissions, and no secret values. | W48,W64 | 11-planning-environment-manifests | 01-step-environment-manifest-creation |

| W68 | source | `planning/scripts/plan-env.sh` | `safe environment manifest consumption` | `N/A` | Provide an explicit helper for locating, validating, and safely sourcing the global and plan-local manifests for trusted temporary scripts; reject missing, malformed, foreign-root, or unsafe manifests. | W67,W65 | 11-planning-environment-manifests | 02-step-environment-manifest-consumption |

| W69 | test | `planning/tests/test-plan-env.sh` | `environment manifest fixture tests` | `N/A` | Test creation, refresh, quoting, permissions, variable completeness, plan isolation, safe sourcing, malformed input rejection, and use from a temporary helper script. | W67,W68 | 11-planning-environment-manifests | 03-step-environment-manifest-tests |

| W70 | docs | `planning/SKILL.md` | `environment contract evolution protocol` | `N/A` | Require coordinated producer, consumer, package, test, validator, and adversarial-review updates for manifest changes; prohibit backward-compatible aliases, adapters, legacy modes, and inferred defaults. | W69 | 12-environment-contract-evolution | 01-step-contract-protocol |

| W71 | test | `planning/tests/test-plan-env.sh` | `schema rejection fixture` | `N/A` | Prove unknown or stale manifest schema state fails before sourcing and does not execute injected content. | W70 | 12-environment-contract-evolution | 02-step-contract-tests |

| W72 | source | `planning/REVIEWER.md` | `generated reviewer projection` | `N/A` | Regenerate the reviewer projection from the authoritative skill and update its source hash. | W70 | 13-reviewer-projection-synchronization | 01-step-regenerate-reviewer |

| W73 | test | `planning/tests/test-reviewer-projection.sh` | `reviewer projection consistency test` | `N/A` | Fail on stale source hash or missing required reviewer sections and verify deterministic regeneration. | W72 | 13-reviewer-projection-synchronization | 02-step-reviewer-sync-tests |

| W74 | source | `benchmark/planning/setup-benchmark.sh` | `publication staging copy` | `N/A` | Exclude `.env` and temporary manifest files from result archives while retaining ordinary plan and reviewer evidence. | W67,W71 | 14-archive-manifest-exclusion | 01-step-filter-archive-manifests |

| W75 | test | `benchmark/planning/tests/test-archive-integrity.sh` | `publication manifest exclusion fixture` | `N/A` | Prove root and nested local manifests are absent from staged archives while required plan evidence remains. | W74 | 14-archive-manifest-exclusion | 02-step-archive-manifest-tests |

| W76 | source | `planning/scripts/plan-env.sh` | `manifest_check` | `N/A` | Reject duplicate keys, expansions, foreign or non-canonical paths, weak ownership, and mismatched derived values before sourcing. | W67,W71 | 15-manifest-path-security | 01-step-harden-manifest-checks |

| W77 | test | `planning/tests/test-plan-env.sh` | `manifest security fixtures` | `N/A` | Test foreign derived paths, mismatched roots, duplicate assignments, expansions, ownership, and symlink rejection. | W76 | 15-manifest-path-security | 02-step-manifest-security-tests |

| W78 | source | `planning/scripts/plan-env.sh` | `validated consumer CLI` | `N/A` | Align the implemented consumer commands and documentation so every supported source flow validates both manifests first. | W76 | 16-manifest-consumer-interface | 01-step-align-consumer-interface |

| W79 | test | `planning/tests/test-plan-env.sh` | `validated consumer fixture` | `N/A` | Prove trusted helpers use one documented interface and reject missing, stale, malformed, and unsafe inputs. | W78 | 16-manifest-consumer-interface | 02-step-consumer-interface-tests |

| W80 | source | `benchmark/planning/setup-benchmark.sh` | `context fixture compatibility block` | `N/A` | Remove or strictly isolate the legacy home-directory fallback and skip path from active benchmark execution. | W71 | 17-no-legacy-fallback | 01-step-remove-legacy-fallback |

| W81 | test | `benchmark/planning/tests/test-safeguards.sh` | `strict fixture configuration test` | `N/A` | Prove unset or missing required fixtures fail and cannot produce successful benchmark status. | W80 | 17-no-legacy-fallback | 02-step-no-fallback-tests |

| W82 | source | `planning/V27-PACKAGE-MANIFEST.txt` | `regression package rows` | `N/A` | Add intended regression files to package records with matching ownership and exclude results and local manifests. | W72,W75,W77,W79,W81 | 18-package-regression-coverage | 01-step-package-regression-files |

| W83 | test | `planning/tests/test-installer-manifest.sh` | `package regression resolution test` | `N/A` | Prove new regression files emit and resolve to source, with package drift and local-manifest exclusion checks. | W82 | 18-package-regression-coverage | 02-step-package-regression-tests |

| W84 | verification | `N/A` | `planning skill and plan implementation-to-plan comparison` | `N/A` | Compare the entire planning skill and plan code/files on disk to scope, acceptance criteria, dependencies, package boundary, tests, and exclusions; classify every discrepancy. | W72,W74,W76,W78,W80,W82 | 19-final-code-plan-review | 01-step-code-plan-comparison |

| W85 | verification | `N/A` | `user disposition record` | `N/A` | Ask the user whether each material gap should amend the plan or remain an explicit exception, then validate any requested amendment. | W84 | 19-final-code-plan-review | 02-step-user-decision-gate |

| W86 | source | `benchmark/planning/seed-blinded-defects.sh` | `encrypted seeded-defect fixture creation` | `N/A` | Create an isolated defective copy, encrypt the opaque defect mapping, record hashes, and keep plaintext/key outside target-visible inputs. | W58,W80 | 20-blinded-seeded-defect-oracle | 01-step-seed-encrypted-defects |

| W87 | source | `benchmark/planning/setup-benchmark.sh` | `blinded target launch boundary` | `N/A` | Run targets without the defect key/map, preserve role/session separation and lifecycle evidence, and hand immutable evidence to the independent oracle. | W86,W57 | 20-blinded-seeded-defect-oracle | 02-step-target-isolation |

| W88 | source | `benchmark/planning/review-oracle.sh` | `independent blinded-run grading` | `N/A` | Decrypt only after terminal target evidence, verify hashes and role separation, classify all defect outcomes, and write a report without publishing secrets. | W86,W87,W60 | 20-blinded-seeded-defect-oracle | 03-step-independent-oracle-report |

| W89 | test | `benchmark/planning/tests/test-review-oracle.sh` | `blinded seeded-defect protocol fixtures` | `N/A` | Test encrypted seeding, key/map secrecy, role separation, hash mismatch, incomplete evidence, complete iterative/fresh classifications, cleanup, and report validation. | W86,W87,W88 | 20-blinded-seeded-defect-oracle | 04-step-blinded-oracle-tests |

| W90 | docs | `planning/SKILL.md` | `helper-only plan mutation protocol` | `N/A` | Require all durable .plans additions, edits, approvals, and progress updates to use validated helpers and prohibit direct file edits. | W84 | 21-plan-mutation-helper-governance | 01-step-plan-mutation-protocol |

| W91 | source | `planning/scripts/plan-mutate.sh` | `mutation dispatcher` | `N/A` | Provide the canonical helper dispatcher for goal, work-unit, testing-companion, progress, status, review, and validation mutations. | W90 | 21-plan-mutation-helper-governance | 02-step-plan-mutation-dispatcher |

| W92 | test | `planning/tests/test-progress-helpers.sh` | `helper regression fixtures` | `N/A` | Test helper-only mutations, four-column progress updates, testing-companion creation, and validation dispatch. | W90,W91 | 21-plan-mutation-helper-governance | 03-step-plan-mutation-helper-tests |

| W93 | source | `planning/scripts/create-step-testing.sh` | `Create testing companions through the canonical dispatcher` | `N/A` | Provide atomic helper-only companion creation with strict step validation | W91 | 21-plan-mutation-helper-governance | 04-step-companion-helper |

| W94 | source | `planning/scripts/rebuild-plan-progress.sh` | `Rebuild plan-level progress from goal trackers` | `N/A` | Provide atomic aggregate progress rebuild after durable mutations | W91 | 21-plan-mutation-helper-governance | 05-step-plan-progress-rebuild |

| W95 | source | `planning/scripts/add-adversarial-finding.sh` | `Add adversarial findings through a validated helper` | `N/A` | Provide atomic review-finding insertion with explicit status handling | W91 | 21-plan-mutation-helper-governance | 06-step-adversarial-finding-helper |

| W96 | test | `planning/tests/test-progress-helpers.sh` | `Exercise all helper-only mutation paths` | `N/A` | Cover companion, progress, rebuild, finding, and dispatcher behavior | W90,W91,W93,W94,W95 | 21-plan-mutation-helper-governance | 07-step-helper-regression-coverage |

| W97 | source | `planning/scripts/add-goal.sh` | `Create-plan progress bootstrap` | `N/A` | Create missing plan progress through the canonical helper before aggregate rebuild | W91 | 21-plan-mutation-helper-governance | 08-step-add-goal-bootstrap |

| W98 | source | `benchmark/planning/grade-blinded-run.sh` | `Independent blinded-run grader` | `N/A` | Decrypt and classify retained target evidence in a separate oracle process | W86,W87 | 20-blinded-seeded-defect-oracle | 05-step-independent-grader |

| W99 | test | `planning/tests/test-plan-commands.sh` | `Plan helper lifecycle regression` | `N/A` | Verify add-goal bootstraps progress and all plan mutations remain valid | W97 | 21-plan-mutation-helper-governance | 09-step-plan-helper-regression |

| W100 | test | `benchmark/planning/tests/test-review-oracle.sh` | `Independent grader regression` | `N/A` | Verify the standalone grader is covered by blinded protocol tests | W98 | 20-blinded-seeded-defect-oracle | 06-step-independent-grader-regression |



## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
