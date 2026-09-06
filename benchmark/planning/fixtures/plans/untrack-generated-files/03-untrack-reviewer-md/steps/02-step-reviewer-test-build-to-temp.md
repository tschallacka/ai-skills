# Step: 02-step-reviewer-test-build-to-temp

## Ownership

- Goal: `03-untrack-reviewer-md`
- Work unit: `W18`
- Type: `test`

## Change target

- File: `planning/tests/test-reviewer-projection.sh`
- Primary symbol or file scope: `freshness assertions`
- Subscope: `N/A`

## Objective

§ 4.1
Rewrite from compare-against-committed to build-and-verify: run generate-reviewer.sh to a temp output, assert the pinned SKILL.md SHA-256 and required markers against it, and byte-compare two fresh runs for determinism; a SKILL.md edit that changes the projection must still fail the test.

## Instructions

§ 5.1
Rewrite planning/tests/test-reviewer-projection.sh: run generate-reviewer.sh to a temp output, assert the pinned Source SHA-256 equals the current planning/SKILL.md hash and the required markers are present - against the temp build, not a committed file - and byte-compare two fresh runs for determinism. Fault-inject: perturb SKILL.md's reviewer section and confirm the SHA assertion fails.

## Acceptance criteria

§ 6.1
The test passes with REVIEWER.md removed from the tree; the injected SKILL.md change fails it naming the stale hash; two fresh runs are byte-identical; no assertion reads a tracked REVIEWER.md path.

## Handoff

§ 7.1
W24's probe relies on this test passing without a tree copy of REVIEWER.md.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
