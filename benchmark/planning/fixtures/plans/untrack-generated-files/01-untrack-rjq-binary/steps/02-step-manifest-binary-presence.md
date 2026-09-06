# Step: 02-step-manifest-binary-presence

## Ownership

- Goal: `01-untrack-rjq-binary`
- Work unit: `W02`
- Type: `test`

## Change target

- File: `tests/test-skill-files-manifest.sh`
- Primary symbol or file scope: `platform-conditional binary presence rule`
- Subscope: `N/A`

## Objective

§ 4.1
A listed bin/<triple> row resolves as optional-on-disk: when the file exists it must be a regular executable, when absent the row is skipped without failure, because the install-time notice is the enforcement point and CI release builds are the delivery path.

## Instructions

§ 5.1
Master b0e31f7 already made tests/test-skill-files-manifest.sh skip absent planning/bin/* rows (cross-target artifacts are CI outputs) and gave plan-overview rows their unshipped reasons. Verify that rule covers the rjq rows after W01 untracks the blob, and add the missing direction: when a bundled binary exists but is invalid (non-executable, or an architecture that cannot match its condition), the test must still fail. Run the test with the binary present, absent, and invalid.

## Acceptance criteria

§ 6.1
Absent rjq binary: test passes via master's skip rule; present-and-valid: passes; present-but-invalid: fails naming the row; the plan-overview skip reasons are untouched.

## Handoff

§ 7.1
W03 and W34 can rely on the manifest test tolerating an absent binary, so the install-time notice is the sole enforcement point.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
