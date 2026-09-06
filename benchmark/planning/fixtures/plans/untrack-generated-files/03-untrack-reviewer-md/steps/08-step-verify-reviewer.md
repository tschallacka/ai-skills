# Step: 08-step-verify-reviewer

## Ownership

- Goal: `03-untrack-reviewer-md`
- Work unit: `W24`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `untrack-reviewer end-to-end probe`
- Subscope: `N/A`

## Objective

§ 4.1
Remove REVIEWER.md from a scratch checkout: role-context.sh fails with the named fix; test-reviewer-projection.sh passes against fresh builds; a build-release.sh tarball contains a generated REVIEWER.md; the capsule contains one on both assembly paths.

## Instructions

§ 5.1
In a scratch checkout with REVIEWER.md removed: (1) role-context.sh fails naming the generator; (2) planning/tests/test-reviewer-projection.sh passes; (3) installer/build-release.sh produces a tarball containing a generated REVIEWER.md; (4) setup-benchmark.sh assembles a capsule containing one on both paths. Capture each assertion's output.

## Acceptance criteria

§ 6.1
All four assertions hold; the scratch tree's git status shows REVIEWER.md only as an untracked-ignored file.

## Handoff

§ 7.1
W31's whole-plan proof repeats the no-tree-copy behaviour after all goals land.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
