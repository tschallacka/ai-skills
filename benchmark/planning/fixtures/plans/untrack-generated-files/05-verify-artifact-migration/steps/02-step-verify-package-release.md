# Step: 02-step-verify-package-release

## Ownership

- Goal: `05-verify-artifact-migration`
- Work unit: `W32`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `package and release-surface probe`
- Subscope: `N/A`

## Objective

§ 4.1
npm pack --dry-run contents assert five libs and REVIEWER.md present, zero lib sources, no planning/bin path; installer/build-release.sh tarball asserts the same and a scratch install from it delivers a working skill set.

## Instructions

§ 5.1
In the working tree: npm pack --dry-run and assert the package contains the five compiled libs and a REVIEWER.md whose pinned hash matches planning/SKILL.md, zero lib/ sources, zero build-plan-libs.sh, and no planning/bin path. Then installer/build-release.sh, assert the same contents of the tarball, and run a scratch install from it expecting completion (with the rjq notice when no binary is bundled).

## Acceptance criteria

§ 6.1
Package and tarball content assertions all hold; the scratch install exits 0 and the installed skill tree contains the five libs and REVIEWER.md; test-release-package.sh passes in the working tree.

## Handoff

§ 7.1
No downstream reliance: the assertions are the packaging proof T70 builds on.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
