# Step: 01-step-install-sh

## Ownership

- Goal: `05-update-consumers`
- Work unit: `W21`
- Type: `config`

## Change target

- File: installer/src/50-manifest.sh
- Primary symbol or file scope: skill_files chat block in installer source
- Subscope: `N/A`

## Objective

§ 4.1
Update the installer SOURCE — installer/src/50-manifest.sh (and installer/src/05-config.sh) — so the generated install.sh skill_files() chat block lists the prebuilt rust server and client binaries under chat/bin/ (BUILT BY THE RELEASE/CI, NEVER BY THE SHIPPED INSTALL.SH, and not committed), with NO crate-build step in skill_files() (install.sh never runs cargo: a released payload has no src/ and no cargo on the target). SKILL_NAMES/description text reflects rust-only chat. Regenerate install.sh with installer/build.sh afterwards.

## Instructions

§ 5.1
<direct action on this one target>

## Acceptance criteria

§ 6.1
<observable result for this target>

## Handoff

§ 7.1
<what the next named work unit can rely on>

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
