# Step: 07-step-maintainer-reviewer-doc

## Ownership

- Goal: `03-untrack-reviewer-md`
- Work unit: `W23`
- Type: `docs`

## Change target

- File: `planning/MAINTAINER.md`
- Primary symbol or file scope: `section 1 REVIEWER.md row`
- Subscope: `N/A`

## Objective

§ 4.1
Align the artifact-map REVIEWER.md row with the untracked reality: generated on demand by generate-reviewer.sh, pinned to SKILL.md's hash, never committed.

## Instructions

§ 5.1
In planning/MAINTAINER.md section 1, align the REVIEWER.md artifact-map row: generated on demand by planning/scripts/generate-reviewer.sh, pinned to planning/SKILL.md's hash, never committed; consumers generate-or-refuse per this plan.

## Acceptance criteria

§ 6.1
The row no longer implies a committed file and names the generator; section 2.15's named-files list stays consistent with it.

## Handoff

§ 7.1
No downstream reliance: the row is the maintainer-facing record.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
