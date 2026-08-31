# Step: 02-step-toolchain-pin

## Ownership

- Goal: `17-build-toolchain`
- Work unit: `W107`
- Type: `config`

## Change target

- File: `src/plan-overview/rust-toolchain.toml`
- Primary symbol or file scope: `toolchain pin`
- Subscope: `N/A`

## Objective

§ 4.1
Pin src/plan-overview/rust-toolchain.toml to Rust 1.86.0 with the artifact matrix targets and components and mark it MODE: DEV.

## Instructions

§ 5.1
Create src/plan-overview/rust-toolchain.toml with channel 1.86.0 and the target components required for x86_64 and aarch64 Linux musl macOS and x86_64 Windows msvc builds. Mark MODE: DEV alone on its first line. Record the exact pinned version 1.86.0 in the step. This file only.

## Acceptance criteria

§ 6.1
Building the crate in a shell whose default toolchain differs from the pin uses rustc 1.86.0 and the required targets. The marker gate recognises MODE alone and does not demand PACKAGE.

## Handoff

§ 7.1
W103's flake declaration and this pin name the same floor, so the dev shell and the crate agree rather than each asserting a version the other does not read. W108 has the toolchain file it checks the marker of.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
