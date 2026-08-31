# Step: 03-step-repo-gates

## Ownership

- Goal: `09-verification`
- Work unit: `W48`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `repository gates on both shells`
- Subscope: `N/A`

## Objective

§ 4.1
Join the terminal proof after the removal fixture registration toolchain package and runtime-isolation branches complete. Run both shells with the crate suite and all gates and reject an early or unconfigured run.

## Instructions

§ 5.1
Run the two-shell harness once after the removal branch W15 W16 W78 W79 W80 W81 W82 W83 W84 W85 W86 W87 W88 and W89; after the fixture and registration branch W91 W92 W93 W94 W95 W96 W97 W98 W99 W110 W114 and W115; after the toolchain branch W103 W107 W112 and W116; and after the package and isolation branch W104 W105 W106 W111 W113 W117 W118 W119 and W120. The run must execute the crate tests and all shell gates and must fail if any prerequisite is absent or unconfigured.

## Acceptance criteria

§ 6.1
Every gate passes on both shells only after the terminal prerequisite set is complete. The crate suite executes with rustc 1.86.0; the generated installer matches its source; marker and skill-files gates pass; and the portability catalogue is unchanged or deliberately regenerated. A run that starts before any listed branch is complete or that omits the crate suite is not a pass.

## Handoff

§ 7.1
Without this the removals cannot be defended as safe. The dependency list is deliberately long and explicit: adversarial finding AR-14 recorded that this unit previously depended only on W75, W76, W15 and W16, so goals 13, 14 and 15 were in none of its ancestor sets and the gate would have run before the removals it must observe, on a tree where the suite still drove deleted files and the manifest still listed them.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
