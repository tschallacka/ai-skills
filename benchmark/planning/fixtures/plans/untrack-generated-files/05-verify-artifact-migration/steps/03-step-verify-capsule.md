# Step: 03-step-verify-capsule

## Ownership

- Goal: `05-verify-artifact-migration`
- Work unit: `W33`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `benchmark capsule tag-path smoke`
- Subscope: `N/A`

## Objective

§ 4.1
Run setup-benchmark.sh against a tag checkout so the git-archive path assembles the capsule; assert built libs and a generated REVIEWER.md inside it.

## Instructions

§ 5.1
Precondition: prior goals are committed to HEAD; create a temp tag at that commit if none exists (git tag tmp-t72-verify) and archive from it, so the git-archive path is exercised for real. Run benchmark/planning/setup-benchmark.sh against the tag checkout and assert the capsule contains the five built libs and a REVIEWER.md matching the capsule's SKILL.md hash; repeat once against the live tree. Do not run a benchmark - capsule assembly only.

## Acceptance criteria

§ 6.1
Both capsules are complete: five libs present and sourceable, REVIEWER.md present with matching pin; neither capsule contains build-plan-libs.sh beyond what the manifest's capsule copy dictates (the existing capsule manifest decides that).

## Handoff

§ 7.1
No downstream reliance: the capsule proof is the benchmark harness's guarantee.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
