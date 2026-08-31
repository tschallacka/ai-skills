# Step: 07-step-crate-manifest

## Ownership

- Goal: `01-engine-core`
- Work unit: `W77`
- Type: `config`

## Change target

- File: `src/plan-overview/Cargo.toml`
- Primary symbol or file scope: `crate manifest`
- Subscope: `N/A`

## Objective

§ 4.1
Create the crate manifest for the renderer with no runtime dependencies and a named test-per-field-buffer feature used only by W117's deterministic mutation test. The crate sits at src/plan-overview, one directory per binary under src, as CODE-STYLE section 1b requires.

## Instructions

§ 5.1
Create src/plan-overview/Cargo.toml with the renderer package metadata, release profile, and no external runtime dependencies. Declare the named test-per-field-buffer feature that W117 uses for its deterministic allocation mutation; it is test-only and does not add a runtime dependency.

## Acceptance criteria

§ 6.1
Cargo parses the manifest, the release profile is present, the dependency list is empty, and the test-per-field-buffer feature is declared without adding a runtime dependency.

## Handoff

§ 7.1
W100 can rely on a crate that exists and builds, so the module tree has somewhere to live; W107 has a crate directory to pin a toolchain in; W17 can rely on the profile the shipped artifacts are built with.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
