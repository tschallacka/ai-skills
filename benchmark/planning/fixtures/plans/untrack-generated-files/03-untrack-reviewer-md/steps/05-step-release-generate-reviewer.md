# Step: 05-step-release-generate-reviewer

## Ownership

- Goal: `03-untrack-reviewer-md`
- Work unit: `W21`
- Type: `source`

## Change target

- File: `installer/build-release.sh`
- Primary symbol or file scope: `collect() precondition`
- Subscope: `reviewer generation`

## Objective

§ 4.1
Run planning/scripts/generate-reviewer.sh before collect when REVIEWER.md is missing from the tree being packaged, mirroring the library build-if-missing step.

## Instructions

§ 5.1
In installer/build-release.sh, after the library build-if-missing and before collect(), run planning/scripts/generate-reviewer.sh when REVIEWER.md is missing from the tree being packaged; generation depends on the compiled plan-crypt-lib.sh, so the library step must have run first (dependency W12 is what guarantees it). A present file is left alone - the projection test owns staleness.

## Acceptance criteria

§ 6.1
A release build from a tree without REVIEWER.md succeeds and the tarball contains a generated one whose pinned hash matches the packaged SKILL.md; with the file present the generator does not run (mtime unchanged).

## Handoff

§ 7.1
W32's tarball pin assertion relies on this generation.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
