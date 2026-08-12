# Goal: Enforce isolated worker capsules and access controls

## Current state and prior-goal handoffs

§ 2.1
setup-benchmark.sh currently archives tagged source and grants the worker --add-dir access to the full tagged source root; prompts rely on instructions rather than a minimal capsule.

## Outcome and definition of done

§ 3.1
Give every worker and fresh reviewer a minimal filesystem capsule, preserve approved-relative-reference resolution, and taint unauthorized access attempts.

## Why this goal is needed

§ 4.1
Prompt allowlists cannot enforce the 1.4.2 safety boundary, so the harness must physically scope worker and reviewer inputs and audit escape attempts.

## Scope

§ 5.1
Include per-worker and per-fresh-review capsule creation, resolved relative references, .bm-vars/wrappers, prompt allowlists, and access tests. Exclude protocol archival and context implementation details.

## Affected files, systems, data, and interfaces

§ 6.1
Change benchmark/planning/setup-benchmark.sh, worker-prompt.md, analyzer-prompt.md, and add bounded capsule/access fixtures under benchmark/planning tests.

## Dependencies and handoffs

§ 7.1
Consume Goal 1 contract and Goal 2 lifecycle/session boundaries. Hand off capsule paths, audit records, and fresh-review isolation guarantees to Goals 4–7.

## Implementation approach, risks, and edge cases

§ 8.1
Use separate capsule, workspace, and result roots; create fresh capsules for fresh reviewers; keep validator scripts outside capsules unless explicitly needed; treat broad repository searches and parent-directory access as taint evidence.

## Owned work units

§ 9.1
`W11` — Build a clean per-worker capsule containing only task inputs, tagged planning skill, REVIEWER.md for reviewers, and resolved relative references; keep capsule, workspace, and archive roots separate.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal changes executable benchmark or planning behavior and owns focused tests or verification units in the inventory. |

§ 9.2
`W12` — Launch workers with capsule-only filesystem scope, create per-worker .bm-vars and wrapper paths, and record command/path access attempts for tainting.

§ 9.3
`W13` — Constrain worker reads to the capsule and workspace, forbid repository history/installed skills/previous results, and require explicit escape reporting.

§ 9.4
`W14` — Constrain analyzers to the run instructions, summary, and current result archive while preserving access-audit evidence and taint semantics.

§ 9.5
`W15` — Verify allowed files are readable, unallowlisted source and prior result roots are unavailable, and escape attempts create tainted evidence.

§ 9.6
`W43` — Enumerate every relative reference required by SKILL.md/REVIEWER.md and determine the minimal capsule file set before implementing the capsule copy boundary.

§ 9.7
`W44` — Replace full tagged-source --add-dir access with the worker capsule and explicitly expose only the workspace plus approved capsule paths; record command/path audits.

§ 9.8
`W45` — Create a fresh analyzer capsule containing only benchmark instructions, summary, and current run results, with no source checkout or previous result roots.

§ 9.9
`W46` — Test physical availability of allowed files and denial/taint for broad source, installed skills, parent paths, previous results, and unapproved validator scripts.

## Goal-size exception

§ 11.1
Not applicable: this goal owns multiple work units and does not use the single-unit exception.
