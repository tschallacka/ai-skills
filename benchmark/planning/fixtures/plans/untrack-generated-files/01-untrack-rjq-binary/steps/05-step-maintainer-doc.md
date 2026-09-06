# Step: 05-step-maintainer-doc

## Ownership

- Goal: `01-untrack-rjq-binary`
- Work unit: `W05`
- Type: `docs`

## Change target

- File: `planning/MAINTAINER.md`
- Primary symbol or file scope: `section 1 binaries.tsv row and section 2.8a binary-path wording`
- Subscope: `N/A`

## Objective

§ 4.1
State that per-target artifacts are CI-delivered and untracked, and that a local planning/bin/<triple> path exists only after a local build; align the section 1 artifact-map row with the untracked reality.

## Instructions

§ 5.1
In planning/MAINTAINER.md, align the section 1 artifact-map wording for binaries.tsv and the section 2.8a sentences that reference planning/bin/<target triple> paths: per-target artifacts are CI-delivered and untracked; a local planning/bin/<triple> path exists only after a local build; cross-reference section 2.15. No other section changes.

## Acceptance criteria

§ 6.1
No remaining wording in sections 1 or 2.8a claims per-target binaries are tracked; the row and the binary-path sentences both point at section 2.15; the file still passes any mode-marker or section-shape checks that apply to it.

## Handoff

§ 7.1
Goal 03's MAINTAINER row edit (W23) assumes section 1 already reflects the CI-delivered binary story.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
