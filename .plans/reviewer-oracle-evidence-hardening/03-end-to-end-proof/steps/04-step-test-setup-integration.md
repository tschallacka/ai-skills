# Step: 04-step-test-setup-integration

## Ownership

- Goal: `03-end-to-end-proof`
- Work unit: `W14`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-review-lifecycle.sh`
- Primary symbol or file scope: `setup-benchmark integration fixture`
- Subscope: `N/A`

## Objective

§ 4.1
Exercise deterministic fake Reviewer B approval and seeded capsule inputs through the setup adapter and assert published evidence/state/provenance artifacts.

## Instructions

§ 5.1
Build a deterministic isolated fixture under `$tmp/reviewer-oracle-fixture`, write the seeded plan/defective plan, capsule metadata, approval.json, transcript, lifecycle metadata, and fake command there, then invoke the real script with its actual four-argument interface: `REVIEWER_COMMAND="$tmp/reviewer-oracle-fixture/fake-reviewer.sh" REVIEWER_SESSION_ID=20260811T000000Z-adapter-test-current-B-fixed REVIEWER_CAPSULE_ID=capsule-001 REVIEWER_MODE=fresh-review REVIEWER_APPROVED_AT=2026-08-11T00:00:00Z BLINDED_ORACLE_SPEC="$tmp/reviewer-oracle-fixture/seeded-defects.json" bash benchmark/planning/setup-benchmark.sh current "$tmp/reviewer-oracle-fixture/testing" adapter-test 20260811T000000Z-adapter-test`. W16 requires the adapter to use and echo those deterministic reviewer inputs when the test seam is enabled, so W15 binds the same session/capsule/mode/timestamp that the fake command writes. Inspect these exact generated paths: `benchmark/results/20260811T000000Z-adapter-test/current/oracle-terminal-evidence.json`, `oracle.json`, `reviewer-state.json`, `protocol-metadata.json`, `reviewer-lifecycle.jsonl`, `reviewers/<session>/plan/approval.json`, `reviewers/<session>/workspace/reviewer.jsonl`, and `evaluation.md`; do not call grade-blinded-run.sh directly. W16 owns the injection seam and deterministic identity inputs used by this invocation.

## Acceptance criteria

§ 6.1
The fixture proves the actual adapter preserves each of finding_id, path, location, summary, observed_contradiction, impact, evidence, required_correction, and independent byte-for-byte in `oracle-terminal-evidence.json`; `oracle.json` reports 3/3 semantic and independent catches; `reviewer-state.json` selects B and records the same session/capsule/mode; protocol-metadata hashes match the named files; reviewer lifecycle and transcript paths exist; and private seed material is absent from public archive output. A malformed fixture also matches W02’s exact canonical JSON shape, fixed reason mapping, and `adoption: false`.

## Handoff

§ 7.1
W10 consumes the integration result and records any bounded failure as fail-closed.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
