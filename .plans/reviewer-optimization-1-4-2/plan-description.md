# Plan: Implement reviewer optimization and benchmark protocol 1.4.2

## Current state

§ 2.1
The repository HEAD is version 1.4.1 but already contains early 1.4.2-oriented reviewer/profile and v27 context/package machinery, alongside benchmark setup/runner/analyzer scripts, telemetry lookup, and historical result archives. The brainstorm identifies the remaining protocol, access-control, lifecycle, telemetry, archival, and pilot work; this plan distinguishes existing HEAD behavior from the exact tagged benchmark source and names the remaining delta.

## Desired outcome

§ 3.1
Deliver a tested 1.4.2 benchmark protocol and planning-skill package that supports token-efficient iterative reviewer correction while retaining a mandatory fresh independent final review, hard filesystem capsules, bounded context reads, complete lifecycle telemetry, reliable cancellation and archival, honest legacy/new-cohort analysis, and a final cleanup of unnecessary developer-journey comments in implementation code.

## Approach

§ 4.1
Implement in dependency order: establish the authoritative contract and generated reviewer profile; add bounded reviewer lifecycle behavior; enforce capsule/access boundaries; extend context and checkpoint state; transition cohort and archive metadata; harden cancellation, telemetry, taint, and journey reporting; then run focused tests, package checks, and a one-to-two-revision pilot before applying the decision rule.

## Scope

§ 5.1
Included: planning/SKILL.md and generated REVIEWER.md, planning helpers/tests, benchmark prompts/runners/setup/telemetry/schema/tests, and the protocol evidence needed to execute and analyze 1.4.2. The installable package boundary is planning/ plus explicitly listed planning package metadata only; benchmark inputs, result archives, telemetry databases, capsules, and pilot evidence are source-only/runtime artifacts and must not enter the installer manifest unless a later explicit decision adds them.

## Affected areas

§ 6.1
Existing HEAD baseline: planning/SKILL.md already has REVIEWER_SECTION markers; planning/scripts/generate-reviewer.sh already emits planning/REVIEWER.md; planning/scripts/plan-context*.sh already implement Phase 1 bounded plan caching; benchmark/planning/setup-benchmark.sh already generates worker launch/cleanup/evaluation; run-benchmark.sh already selects tags and launches one analyzer; telemetry.sh already performs text-based UUID lookup; planning/V27-PACKAGE-MANIFEST.txt and MAP.tsv already define the installable planning package.

§ 6.2
Target delta: add the named reviewer lifecycle/capsule/checkpoint/schema/archive/control/oracle behavior in W01–W60, preserving existing legacy behavior unless a step explicitly changes the named symbol. Source-only benchmark artifacts are benchmark/planning/*, benchmark/results/*, /tmp/<run-id>/*, and telemetry stores; installable artifacts are only the exact planning/ files enumerated by W26/W27.

§ 6.3
6.3: Canonical runtime roots and contracts are: worker capsule=/tmp/ai-skills-capsules/<run-id>/<revision>/worker; reviewer capsule=/tmp/ai-skills-capsules/<run-id>/<revision>/reviewers/<reviewer-session>; analyzer capsule=/tmp/ai-skills-capsules/<run-id>/analysis; workspace=/tmp/<run-id>/<revision>/workspace; private staging=benchmark/results/.staging/<run-id>/<revision>; published archive=benchmark/results/<run-id>/<revision>; external checkpoints=/tmp/ai-skills-checkpoints/<run-id>/<revision>. The worker launch receives only the worker capsule and workspace; the analyzer receives only its capsule and current published-run input.

§ 6.4
6.4: Mandatory telemetry fields are schema_version, protocol_id, skill_version, run_id, revision, worker_thread_id, reviewer_mode, reviewer_session_id when reviewers run, review_cycle, verification_pass, phase_start/end timestamps, telemetry_source, telemetry_db or rollout provenance, status, and taint_causes. Missing identity/provenance/status is a publication failure; metric fields may be unavailable only when marked with reason and exact/heuristic classification.

§ 6.5
6.5: The pilot runs the current working-tree protocol (`current`) in separate fresh temporary roots, with one fresh-review control and one iterative run for the current protocol, no parallel scheduling, and a seeded oracle fixture. Historical reports remain immutable contextual evidence and are not rerun or retrofitted. Adopt iterative mode only when telemetry is complete, taint is zero, final independent review passes, true-positive and independent-catch rates are not lower than control, and total tokens decrease; otherwise retain fresh mode.

§ 6.6
6.6: Variable/path contract: WORKER_CAPSULE=/tmp/ai-skills-capsules/<run-id>/<revision>/worker (read-only capsule), WORKER_WORKSPACE=/tmp/<run-id>/<revision>/workspace (read/write), REVIEWER_CAPSULE=/tmp/ai-skills-capsules/<run-id>/<revision>/reviewers/<session> (read-only), REVIEWER_WORKSPACE=/tmp/<run-id>/<revision>/reviewers/<session>/workspace (read/write), ANALYZER_CAPSULE=/tmp/ai-skills-capsules/<run-id>/analysis (read-only), ANALYZER_WORKSPACE=/tmp/<run-id>/analysis-<run-id> (read/write), CURRENT_RUN_INPUT=/tmp/ai-skills-capsules/<run-id>/analysis/current-run.tsv, STAGING_DIR=benchmark/results/.staging/<run-id>/<revision>, PUBLISHED_DIR=benchmark/results/<run-id>/<revision>. Worker/reviewer commands receive only their capsule and workspace; analyzer receives only ANALYZER_CAPSULE, ANALYZER_WORKSPACE, and CURRENT_RUN_INPUT.

§ 6.7
6.7: Shared audit event schema is JSONL with required event_id, actor, session_id, timestamp, operation, path, decision, evidence_path, and taint_code. Manifests are worker-manifest.json, reviewer-manifest.json, and analyzer-manifest.json under their respective capsule roots; each contains root, entries[{path,sha256,role}], source_hash, and schema_version. Denied events append to audit.jsonl and map to taint-causes.json before W50 publication checks.

§ 6.8
6.8: Checkpoint schema is JSON with schema_version, run_id, revision, phase enum {drafting,review,correction,validation}, state, open_findings[], next_action, changed_files[], source_hash, plan_hash, created_at, updated_at. `plan-context-lib.sh` writes atomically via `.tmp` plus rename; `setup-benchmark.sh` invokes it after worker drafting/validation, `run-benchmark.sh` invokes it after review/correction, and readers reject mismatched run/revision or stale source/plan hashes.

§ 6.9
6.9: Telemetry producer order is telemetry.sh W53 raw extraction -> telemetry.sh W60 phase/lifecycle augmentation -> telemetry-schema.json final validation -> telemetry-rejection.json on failure or telemetry.json on success -> setup-benchmark.sh W30 evaluation/status synthesis -> W50 publication precondition. Raw paths are <STAGING_DIR>/telemetry/raw.jsonl, <STAGING_DIR>/telemetry/telemetry.json, and <STAGING_DIR>/telemetry/telemetry-rejection.json; no post-validation writer may modify telemetry.json.

§ 6.10
6.10: Launch rows: (1) worker: codex exec -C WORKER_WORKSPACE --add-dir WORKER_CAPSULE --add-dir WORKER_WORKSPACE with worker-prompt.md, output WORKER_WORKSPACE/worker.jsonl; (2) Reviewer A: codex exec -C REVIEWER_WORKSPACE --add-dir REVIEWER_CAPSULE --add-dir REVIEWER_WORKSPACE with reviewer-a-prompt.md, output REVIEWER_WORKSPACE/reviewer.jsonl, inputs changed-files.txt/bounded.diff/targeted-validation.txt; (3) Reviewer B: same command with a new REVIEWER_CAPSULE and REVIEWER_WORKSPACE, full task/spec/skill manifest only, output reviewer-b.jsonl; (4) analyzer: codex exec -C ANALYZER_WORKSPACE --add-dir ANALYZER_CAPSULE --add-dir ANALYZER_WORKSPACE with analyzer-prompt.md and CURRENT_RUN_INPUT, output analyzer.jsonl/comparison.json. No command may add SRC_ROOT or RUN_RESULTS_ROOT directly.

§ 6.11
6.11: Review artifact schemas: changed-files.txt is one normalized relative path plus sha256 per line; bounded.diff begins with source/target hashes and contains only changed-file hunks; targeted-validation.txt is command, exit_code, stdout_sha256, stderr_sha256; approval.json requires reviewer_session_id, mode, approved_findings[], rejected_findings[], approved_at and may not contain overall_plan_approval for Reviewer A; lifecycle JSONL requires event_id, actor, session_id, event_type enum, cycle, verification_pass, timestamp, input_manifest_sha256, output_path, and evidence_paths. Reviewer B manifest is reviewer-b-manifest.json with full task/skill/reference entries and an explicit excluded_paths array containing every A artifact and session path.

§ 6.12
6.12: Audit/taint mapping: audit decision=deny on a worker/reviewer/analyzer path yields BB-ACCESS-DENIED; process audit residue yields BB-PROCESS-RESIDUE; missing/ambiguous telemetry yields BB-TELEMETRY-INTEGRITY; reviewer/analyzer nonzero yields BB-REVIEWER-FAILURE/BB-ANALYZER-FAILURE; validation failure yields BB-VALIDATION-FAILURE. Each cause stores the originating audit event IDs and evidence path. W50 verifies audit.jsonl, derives/validates taint-causes.json, writes evaluation.md, then checks telemetry schema, then checks required archive files, then atomically renames.

§ 6.13
6.13: Checkpoint field types: schema_version/run_id/revision/state/next_action are non-empty strings; phase is the four-value enum; open_findings/changed_files are arrays of strings; source_hash/plan_hash are 64-hex SHA-256 strings; timestamps are UTC RFC3339. Required state is {in_progress, blocked, complete}; stale or identity mismatch writes checkpoint-rejection.json and exits 65. Callers are setup-benchmark.sh start/end worker phases, run-benchmark.sh after reviewer A and correction, and plan-context.sh check --changed before every read.

§ 6.14
6.14: Telemetry top-level JSON requires schema_version/protocol_id/skill_version/run_id/revision/worker_thread_id/reviewer_mode/status/taint_causes/telemetry_source/provenance and phase_records. Provenance is either database_path plus exact UUID lookup or rollout_path plus exact thread extraction. Reviewer records are required for every reviewer session; metrics are objects with value nullable, status exact|heuristic|unavailable, and reason required when nullable. Raw/rejection expansion is benchmark/results/.staging/<run-id>/<revision>/telemetry/{raw.jsonl,telemetry.json,telemetry-rejection.json}; telemetry.sh produces raw, validator produces valid or rejection, setup-benchmark.sh consumes valid, W50 blocks on rejection.

§ 6.15
6.15: Complete telemetry item types: phase_records are objects with phase enum {setup,capsule,drafting,review,correction,fresh_review,validation,archive,analysis}, start/end RFC3339, duration_seconds nonnegative number, source enum {runner,event,db,rollout}, and precision enum {exact,heuristic,unavailable}; reviewer_records require session_id/cycle/verification_pass strings or nonnegative integers plus event_count/token_total and source; taint_causes require code enum, layer enum, evidence_paths array, and event_ids array; status enum is accepted|tainted|rejected|interrupted. Raw JSONL is producer-only, telemetry.json is validator output, telemetry-rejection.json records schema errors and source line hashes.

§ 6.16
6.16: comparison.json at `/tmp/<analysis-run-id>/comparison.json` requires protocol_id, task_spec_sha256, archives[{mode,revision,run_id,evaluation_path,telemetry_path,oracle_path,review_approval_path}], cohort_labels, token_deltas, latency_deltas, oracle_rates, taint_summary, threshold_results, final_independent_gate, adoption, and unavailable_reasons. The analyzer must validate this object before rendering comparison.md; W38 rejects missing, malformed, or non-machine-readable comparison.json.

§ 6.17
6.17: Package equality uses four sorted files: `manifest.sources`, `map.sources`, `installer.sources`, and `generated.sources`; each must be byte-identical. `installer.sources` comes from `install.sh --print-skill-files planning --format=tsv`, `generated.sources` is the generated REVIEWER.md plus helper outputs named in the manifest, and an exclusion assertion rejects benchmark/planning, benchmark/results, telemetry stores, capsules, and pilot reports.

§ 6.18
6.18: Complete launch table: worker prompt=`WORKER_CAPSULE/worker-prompt.md`, inputs=WORKER_CAPSULE/task-spec.md plus manifest entries, output=`WORKER_WORKSPACE/worker.jsonl`; Reviewer A prompt=`REVIEWER_CAPSULE/reviewer-a-prompt.md`, inputs=`REVIEWER_WORKSPACE/changed-files.txt`, `bounded.diff`, `targeted-validation.txt`, output=`REVIEWER_WORKSPACE/reviewer-a.jsonl` and `reviewer-handoff.json`; Reviewer B prompt=`REVIEWER_CAPSULE/reviewer-b-prompt.md`, inputs=`REVIEWER_CAPSULE/task-spec.md`, tagged skill/reference entries, `reviewer-b-manifest.json`, output=`REVIEWER_WORKSPACE/reviewer-b.jsonl` and `approval.json`; analyzer prompt=`ANALYZER_CAPSULE/analyzer-prompt.md`, input=`ANALYZER_CAPSULE/current-run.tsv` plus copied four archive manifests, output=`ANALYZER_WORKSPACE/analyzer.jsonl`, `comparison.json`, and rendered `comparison.md`. Every command uses its actor workspace for -C, exactly its capsule and workspace for --add-dir, and copies outputs into STAGING_DIR before W50 publication.

§ 6.19
6.19: Text schemas: changed-files.txt is UTF-8 LF records `<normalized-relative-path><TAB><64-hex-sha256>` with no blank lines; bounded.diff starts with `source_manifest_sha256=<hash>` and `target_manifest_sha256=<hash>` then unified-diff hunks for only those paths; targeted-validation.txt is LF records `<command-sha256><TAB><exit-code><TAB><stdout-sha256><TAB><stderr-sha256>`. JSON fields are typed: handoff IDs/mode/paths are strings, cycle/pass are nonnegative integers, finding arrays are string IDs, closed_findings is array of objects `{id,owner,verification_pass,closed_at}`, and approval has required session/mode/approved_findings/rejected_findings/approved_at with boolean `overall_plan_approval` forbidden for A and required false for B until final-gate synthesis.

§ 6.20
6.20: Manifest/audit retention: worker-manifest.json at WORKER_CAPSULE, reviewer-a-manifest.json and reviewer-b-manifest.json at each per-session REVIEWER_CAPSULE, analyzer-manifest.json at ANALYZER_CAPSULE; each entry is `{path:string,sha256:64hex,role:enum}`. Each actor writes `<STAGING_DIR>/audit/<actor>/<session>/audit.jsonl` with `{event_id:string,actor:enum,session_id:string,timestamp:RFC3339,operation:enum,path:string,decision:allow|deny,evidence_path:string,taint_code:string|null}`. W50 copies manifests/audits/taint-causes.json into PUBLISHED_DIR and validates all hashes; A/B manifests are separate and B excluded_paths contains every A path/hash/session.

§ 6.21
6.21: Checkpoint open finding item is `{id:string AR-NN,owner:string,state:enum open|in_progress|closed,verification_pass:nonnegative integer,required_evidence:array<string>,updated_at:RFC3339}`. Writer command is `plan-context.sh checkpoint --plan-dir "$PLAN_DIR" --phase <phase> --state <state> --findings-file <path> --changed-files <path> --source-hash <hash> --plan-hash <hash>`; each caller writes its phase file, and `check --changed` rejects identity/hash mismatches with checkpoint-rejection.json containing error_code, expected/actual hashes, phase, and timestamp, exit 65.

§ 6.22
6.22: Telemetry chain is strictly: telemetry.sh writes raw.jsonl; telemetry.sh augments the same in-memory record with W60 phase metrics and reviewer lifecycle; telemetry-schema validator writes final telemetry.json or telemetry-rejection.json; setup-benchmark.sh reads only final telemetry.json to synthesize status; W50 reads status, audit, telemetry, and required-file checks and atomically publishes. No post-validation writer may modify telemetry.json; any W60 failure writes rejection and blocks publication.

§ 6.23
6.23: Any scope discovered during implementation is a plan mutation, not an informal note. The worker must add a linked work-unit inventory row, owning goal and step, testing companion when applicable, progress row, dependencies, and handoff/evidence requirements; run the plan validator and obtain adversarial-review coverage before treating the addition as part of the plan. Monitoring must distinguish intermediate status reports from terminal completion, continue steering active subprocesses until explicit terminal evidence exists, and record blockers instead of stopping at a status-only update.

§ 6.24
6.24: Planning environment manifests are created by the planning skill: a global `~/.plans/.env` exposes stable, non-secret paths such as `PLANS_ROOT`, `PLANNING_SKILL_ROOT`, `PLANNING_SCRIPTS_ROOT`, and `PLANNING_TESTS_ROOT`; each plan receives `.plans/<plan-name>/.env` with `PLAN_ROOT`, `PLAN_NAME`, `PLAN_PROGRESS_FILE`, `PLAN_WORK_UNIT_INVENTORY`, `PLAN_VALIDATION_FILE`, and related plan-local paths. Files use shell-safe quoted assignments, restrictive permissions, deterministic refresh behavior, and are sourced only by explicit trusted helpers or scripts.

§ 6.25
6.25: Environment-manifest variables are a versioned planning-skill interface (`PLAN_ENV_SCHEMA_VERSION`). Adding or changing a variable requires a coordinated schema migration covering the producer, all applicable consumers, package inventory, focused rejection/fixture tests, plan validation, and adversarial-review evidence. This project does not provide backward-compatible aliases, adapters, legacy modes, or inferred defaults: stale, unknown, missing, or schema-mismatched manifests fail closed, and superseded variables are removed from code, documentation, and tests.

## Constraints and decisions

§ 7.1
The planning skill helper scripts own plan-document writes and must be used for durable plan artifacts. The plan is stored at .plans/reviewer-optimization-1-4-2 in this checkout because the execution environment permits workspace writes there. Legacy benchmark results remain frozen; protocol 1.4.2 receives a distinct identifier and fresh reviewer capsules. Fresh review remains the default for new protocol runs.

§ 7.2
Historical reports and archives are immutable evidence from the code and protocol that produced them. The project does not provide backward compatibility: older-version reports are not rerun or retrofitted to modern protocol requirements. Comparisons use archived data as-is only when task, revision, metadata, and evidence boundaries are compatible; otherwise the comparison records the data as unavailable or contextual.

## Risks and open questions

§ 8.1
Open execution risks are Codex telemetry schema variation, process-group behavior across host systems, filesystem isolation differences, the exact legacy analyzer data available for cohort merging, and whether the one-to-two-case pilot has enough signal for adoption. The executor must record unavailable evidence rather than infer it and must stop for user direction if a fix changes the task contract or safety boundary.

## UI classification

- UI affected: no
- Rationale: This initiative changes benchmark tooling, planning artifacts, and access/telemetry behavior; it does not create or change a user-facing UI flow.

## Adversarial review

- Artifact: `adversarial-review.md`
- Status: ✅ approved
