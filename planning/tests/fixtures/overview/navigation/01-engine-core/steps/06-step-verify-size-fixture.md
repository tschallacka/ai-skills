# Step: 06-step-verify-size-fixture

## Ownership

- Goal: `01-engine-core`
- Work unit: `W06`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `derive on the size fixture`
- Subscope: `N/A`

## Objective

§ 4.1
Prove on the plan that currently cannot render at all that the new path has no argument-length limit.

## Instructions

§ 5.1
Run the binary against the checked-in size fixture W92 freezes, whose state is about 337 KB. Record the measured state size, the wall time, and the derived counts. For contrast, record that the existing renderer exits 126 with Argument list too long on the same input. An earlier version of this instruction named the live codegraph-bash-indexing-v2 plan in the plans root; adversarial findings AR-10 and AR-32 recorded that a plans-root helper can change or delete it, so evidence recorded against it is not reproducible by the next reader.

## Acceptance criteria

§ 6.1
Derived values are complete for the size fixture, no argument-length error occurs anywhere in the path, and the recorded evidence includes both the new timing and the old exit code. US-12 and US-53 build on this.

## Handoff

§ 7.1
The recorded size and timing are the baseline W47, W53 and W69 measure against.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
