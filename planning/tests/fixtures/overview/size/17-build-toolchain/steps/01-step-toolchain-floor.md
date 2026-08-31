# Step: 01-step-toolchain-floor

## Ownership

- Goal: `17-build-toolchain`
- Work unit: `W103`
- Type: `config`

## Change target

- File: `flake.nix`
- Primary symbol or file scope: `rust toolchain input`
- Subscope: `N/A`

## Objective

§ 4.1
Pin Rust 1.86.0 in the existing flake toolchain declaration and require the targets and components needed by the artifact matrix.

## Instructions

§ 5.1
Add Rust 1.86.0 to the Rust toolchain flake.nix already declares for the crates under src. Use the same stable 1.86.0 channel and required target components as rust-toolchain.toml. State that exact version in the flake and step. The compiler is a build dependency only and must not appear in a skill runtime requirement.

## Acceptance criteria

§ 6.1
The dev shell provides rustc 1.86.0 and reports that exact version. The crate target list and components needed by the artifact matrix are available. The compiler appears in no skill requires.tsv entry. Matching the crate pin and CI is verified by W116.

## Handoff

§ 7.1
The cargo leg W75 adds to the repository gate has a compiler to run, and a contributor gets one from the dev shell rather than from a failing build.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
