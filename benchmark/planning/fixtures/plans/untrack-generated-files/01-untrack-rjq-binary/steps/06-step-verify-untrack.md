# Step: 06-step-verify-untrack

## Ownership

- Goal: `01-untrack-rjq-binary`
- Work unit: `W06`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `untrack-rjq end-to-end probe`
- Subscope: `N/A`

## Objective

§ 4.1
On the working tree after W01-W05: git ls-files planning/bin is empty; a scratch install from a tree without the binary completes and prints the notice; npm pack --dry-run file list contains no planning/bin path.

## Instructions

§ 5.1
On the working tree, after W01-W05: (1) assert git ls-files planning/bin is empty; (2) copy the tree to a TMPDIR scratch, remove planning/bin/x86_64-unknown-linux-musl/rjq there, run install.sh against a scratch prefix, and expect completion with the W03 notice on stderr; (3) run npm pack --dry-run and assert the file list contains no planning/bin path. Run the install probe under resource-limited-testing/scripts/limited-run.sh.

## Acceptance criteria

§ 6.1
All three assertions hold and their output is captured in the step's execution log; the scratch install exits 0 with the notice; the pack listing is clean.

## Handoff

§ 7.1
Goal 02 reuses the proven untrack-and-reconcile pattern; the probe commands are the template for goal 05's probes.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
