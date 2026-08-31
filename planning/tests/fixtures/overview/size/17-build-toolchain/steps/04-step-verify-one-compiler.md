# Step: 04-step-verify-one-compiler

## Ownership

- Goal: `17-build-toolchain`
- Work unit: `W116`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `one compiler in three places`
- Subscope: `N/A`

## Objective

§ 4.1
Verify rustc 1.86.0 resolves identically in flake dev shell crate build and CI and record a non-zero CI crate test count and required target components.

## Instructions

§ 5.1
Record the resolved rustc version three times: in the dev shell from a crate build in that shell and from the CI job. Each must equal 1.86.0 rather than merely satisfy a floor. Record the exact target components used by the artifact matrix and the CI crate test count as a number.

## Acceptance criteria

§ 6.1
The three recorded compiler versions are exactly rustc 1.86.0 and the CI crate test count is non-zero. A green run with another version or a zero crate count fails this verification.

## Handoff

§ 7.1
Goal 09 can rely on the repository gate failing when a Rust test fails, and goal 16 builds its artifacts with the compiler this goal named.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
