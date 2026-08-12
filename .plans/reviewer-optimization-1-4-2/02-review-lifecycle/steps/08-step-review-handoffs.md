# Step: 08-step-review-handoffs

## Ownership

- Goal: `02-review-lifecycle`
- Work unit: `W41`
- Type: `source`

## Change target

- File: `benchmark/planning/run-benchmark.sh`
- Primary symbol or file scope: `review finding handoff artifacts`
- Subscope: `N/A`

## Objective

§ 4.1
Write changed-file/diff/targeted-validation handoffs, stable AR-NN ownership, closure passes, termination events, and final fresh-review approval artifacts.

## Instructions

§ 5.1
Write artifacts under REVIEWER_WORKSPACE=/tmp/<run-id>/<revision>/reviewers/<session>/workspace: changed-files.txt, bounded.diff, targeted-validation.txt, reviewer-handoff.json, approval.json, reviewer-lifecycle.jsonl. Handoff JSON requires session_id, cycle, verification_pass, finding_ids, changed_files_sha256, diff_sha256, validation_sha256, closed_findings, next_reviewer, terminated_at. Reviewer B manifest excludes these A workspace paths and hashes; W50 copies this complete workspace artifact set into its staged archive.

## Acceptance criteria

§ 6.1
The exact line grammars and JSON types in plan-description sections 6.18–6.20 validate successfully; A and B have distinct prompts/workspaces/manifests; B receives only its full manifest and no A path/hash/session; W50 retains every manifest, audit, handoff, approval, lifecycle, and taint artifact.

## Handoff

§ 7.1
Record the changed target, command/output evidence, and unavailable or environment-specific results for the next dependency.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
