# Step: 07-step-package-map

## Ownership

- Goal: `05-protocol-archive`
- Work unit: `W27`
- Type: `config`

## Change target

- File: `planning/V27-PACKAGE-MAP.tsv`
- Primary symbol or file scope: `source/destination ownership map`
- Subscope: `N/A`

## Objective

§ 4.1
Map every new or changed package file to its install destination and preserve the explicit six-column ownership boundary.

## Instructions

§ 5.1
Generate `/tmp/package-set-manifest.sources`, `/tmp/package-set-map.sources`, `/tmp/package-set-installer.sources`, and `/tmp/package-set-generated.sources`; sort each exact path set and run pairwise diff. Installer sources come from `install.sh --print-skill-files planning --format=tsv`; generated sources are the manifest-declared generated profile/helper outputs; assert all benchmark/runtime/evidence exclusions.

## Acceptance criteria

§ 6.1
The test compares manifest sources, non-source-only map sources, installer-emitted paths from `install.sh --print-skill-files planning --format=tsv`, and generated outputs; all four files must be byte-identical after sorting, and source-only exclusions are asserted.

## Handoff

§ 7.1
Record the changed target, command/output evidence, and unavailable or environment-specific results for the next dependency.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
