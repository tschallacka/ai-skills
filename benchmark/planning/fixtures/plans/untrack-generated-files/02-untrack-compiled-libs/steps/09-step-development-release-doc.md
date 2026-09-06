# Step: 09-step-development-release-doc

## Ownership

- Goal: `02-untrack-compiled-libs`
- Work unit: `W15`
- Type: `docs`

## Change target

- File: `DEVELOPMENT.md`
- Primary symbol or file scope: `release flow steps`
- Subscope: `N/A`

## Objective

§ 4.1
State that npm pack assembles generated artifacts through prepack, and that no generated file is committed ahead of a release.

## Instructions

§ 5.1
In DEVELOPMENT.md, update the release flow steps: npm pack assembles the generated artifacts through prepack, so no generated file is committed ahead of a release, and the pack verification step covers the five libs and REVIEWER.md. Sweep RELEASE.md for the same wording and record the outcome in this step's handoff - if RELEASE.md carries generated-artifact wording, that edit gets its own unit before this goal closes.

## Acceptance criteria

§ 6.1
DEVELOPMENT.md's release steps match the prepack reality; the RELEASE.md sweep outcome (wording found or not) is recorded in the handoff paragraph.

## Handoff

§ 7.1
Goal 04 and goal 06 doc work assumes the release-flow wording is already aligned; the RELEASE.md sweep outcome is recorded here.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
