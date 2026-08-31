# Step: 06-step-crate-markers

## Ownership

- Goal: `09-verification`
- Work unit: `W108`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `marker pair across the crate`
- Subscope: `N/A`

## Objective

§ 4.1
Confirm every file under src/plan-overview carries the marker its kind requires, on the first two lines: MODE: DEV and PACKAGE: PROD on the manifest and the sources, MODE: DEV alone on the toolchain file, and an exemption reason for the generated lock and the built artifacts. Each source unit writes its own markers; this unit is where the whole crate is checked once, after the sources exist.

## Instructions

§ 5.1
Run the marker gate over every file under src/plan-overview and record, per file kind, which marker it carries: the manifest and every source with MODE: DEV and PACKAGE: PROD on their first two lines, rust-toolchain.toml with MODE: DEV alone, and Cargo.lock and the built artifacts reaching an exemption arm with a printed reason. Record any file the gate does not classify at all rather than assuming the default.

## Acceptance criteria

§ 6.1
Every file under the crate is accounted for by name, and the gate passes. Prove the gate still bites over the crate rather than skipping it: strip the marker pair from one source and confirm the failure names that file, and move another source's pair below its module documentation and confirm that file is reported unmarked. A gate that passes under both mutations is not reading the crate.

## Handoff

§ 7.1
The marker requirement CODE-STYLE section 1b places on crate sources has a single place where it is checked as a whole, and W48 depends on this unit so an unmarked source cannot redden the repository gate before anything has checked for it. Adversarial findings AR-15 and AR-37 recorded, in turn, that no unit owned the requirement and that the unit which grades it was not upstream of the gate that fails on it.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
