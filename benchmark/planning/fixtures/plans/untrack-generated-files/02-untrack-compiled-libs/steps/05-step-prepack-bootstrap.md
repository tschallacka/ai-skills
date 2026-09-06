# Step: 05-step-prepack-bootstrap

## Ownership

- Goal: `02-untrack-compiled-libs`
- Work unit: `W11`
- Type: `config`

## Change target

- File: `package.json`
- Primary symbol or file scope: `scripts.prepack`
- Subscope: `N/A`

## Objective

§ 4.1
Prepend the generator bootstrap to prepack: run planning/scripts/build-plan-libs.sh and planning/scripts/generate-reviewer.sh before the existing manifest and register-schema tests, so pack-time artifact generation is one documented seam.

## Instructions

§ 5.1
In package.json, make scripts.prepack run the generator bootstrap before the existing tests: planning/scripts/build-plan-libs.sh, then planning/scripts/generate-reviewer.sh, then the existing test-skill-files-manifest.sh and test-register-schemas.sh. The order is load-bearing: generate-reviewer.sh sources the compiled plan-crypt-lib.sh, so the libs must exist before it runs; register-schema tests need rjq on PATH, which is a host precondition recorded in the risks section, not something prepack can build.

## Acceptance criteria

§ 6.1
With the five libs and REVIEWER.md removed, npm pack --dry-run succeeds and both artifacts exist afterwards; with them present, the generators still run (idempotent, byte-stable) and pack succeeds; package.json remains valid JSON with the files list untouched.

## Handoff

§ 7.1
W21 repeats the libs-before-reviewer order in build-release.sh; W32's pack assertions rely on this chain.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step. VIOLATION: also touched installer/src/00-header.sh
