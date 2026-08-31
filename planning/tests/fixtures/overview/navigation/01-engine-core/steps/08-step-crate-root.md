# Step: 08-step-crate-root

## Ownership

- Goal: `01-engine-core`
- Work unit: `W100`
- Type: `source`

## Change target

- File: `src/plan-overview/src/main.rs`
- Primary symbol or file scope: `module tree`
- Subscope: `N/A`

## Objective

§ 4.1
The crate root: the module declarations that make plan, render, pages, serve and watch reachable, and nothing else. Without it every other source unit names a file the compiler never sees.

## Instructions

§ 5.1
Create src/plan-overview/src/main.rs declaring the modules the crate is built from: plan, render, pages, serve and watch. Declarations only; the argument surface is W101 and each module's contents belong to the unit that names that file. Carry MODE: DEV and PACKAGE: PROD on the first two lines, before any module documentation, and record here that this pair is the convention every file this plan adds under the crate follows — each source unit writes its own, and W108 is where the whole crate is checked once. Adversarial finding AR-37 recorded that W108 graded an outcome no unit was instructed to produce; stating the convention at the crate root, where the first source is written, is what makes the later checks measure something rather than discover it.

## Acceptance criteria

§ 6.1
The crate compiles from a clean checkout with no network access, and cargo reports zero dependencies. This is the first point at which a build is possible at all, because a manifest with no target cannot be built. A crate that has acquired a dependency fails the offline build, which is the property this criterion exists to pin and which every later build re-proves. Every declared module resolves to a file, a source added under the crate without a declaration fails the build rather than being silently ignored, and the marker gate recognises this file.

## Handoff

§ 7.1
Every later source unit has a crate root that reaches its file, so a module written but never declared is a build failure rather than dead code nobody notices.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
