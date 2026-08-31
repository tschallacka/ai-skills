# Step: 03-step-ci-toolchain

## Ownership

- Goal: `17-build-toolchain`
- Work unit: `W112`
- Type: `config`

## Change target

- File: `.github/workflows/ci.yml`
- Primary symbol or file scope: `rust toolchain step`
- Subscope: `N/A`

## Objective

§ 4.1
Install Rust 1.86.0 and the artifact matrix targets in CI before the crate leg and make a missing or mismatched toolchain fail rather than degrade.

## Instructions

§ 5.1
Add a CI toolchain installation step pinned to Rust 1.86.0 with the target components needed by the artifact matrix and set the environment variable that makes the crate leg refuse rather than degrade when no toolchain is present. The gate must report rustc 1.86.0 before running the crate tests.

## Acceptance criteria

§ 6.1
The CI job reports rustc 1.86.0 and the crate leg runs there with a non-zero test count. Removing the installation step or changing its version makes the job fail rather than pass with the crate leg unconfigured.

## Handoff

§ 7.1
W48's criterion that a run without the crate suite is not a pass is enforced by the automated gate rather than by a human remembering to check, and W49's matrix leg has a compiler on the runner.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
