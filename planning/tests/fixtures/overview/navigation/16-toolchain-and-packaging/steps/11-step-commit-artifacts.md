# Step: 11-step-commit-artifacts

## Ownership

- Goal: `16-toolchain-and-packaging`
- Work unit: `W115`
- Type: `data`

## Change target

- File: `planning/bin`
- Primary symbol or file scope: `committed artifacts`
- Subscope: `N/A`

## Objective

§ 4.1
Commit the five prebuilt artifacts the matrix builds, one per target, named by uname pair. The manifest gate requires every file skill_files() promises to exist on disk, so the artifacts must be in the tree rather than fetched at install time, and this is where the repository takes on their committed size as a stated cost rather than an accident.

## Instructions

§ 5.1
Commit the five artifacts under planning/bin, one per declared target, each named by the uname pair binaries.tsv records for it. Record the byte size of each and the total added to the repository. Confirm each committed file is the one the matrix built by comparing its checksum against the registry rather than by trusting the filename.

## Acceptance criteria

§ 6.1
Five artifacts exist on disk under planning/bin, each checksum matches its binaries.tsv row, and the recorded total size matches what the tree gained. Every path skill_files() promises resolves to a file that is present, which is what the manifest gate requires and what a fetch-at-install design would not satisfy.

## Handoff

§ 7.1
W104 has artifacts to select among, W111 has a tree to validate the registry against, and W114 has the paths it registers. Adversarial finding AR-31 recorded that eight units presumed this location while none owned it.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
