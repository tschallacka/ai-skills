# Step: 06-step-binaries-registry

## Ownership

- Goal: `16-toolchain-and-packaging`
- Work unit: `W110`
- Type: `config`

## Change target

- File: `planning/binaries.tsv`
- Primary symbol or file scope: `shipped binaries registry`
- Subscope: `N/A`

## Objective

§ 4.1
Declare per target which artifact the skill ships: the uname pair, the artifact path and its checksum. CODE-STYLE section 1b names this file as how a skill declares what ships; it does not exist in the repository, and this plan is the first thing to ship a binary, so this unit is where the declared mechanism becomes real.

## Instructions

§ 5.1
Create planning/binaries.tsv with one row per shipped target: the uname -s and uname -m pair the row applies to, the artifact path relative to the skill root, and the artifact's checksum. Tab-separated and line-oriented for the same reason requires.tsv is: it must be readable by awk on a machine that has nothing else. Carry a MODE: PROD comment on the first line, as requires.tsv does. An earlier version of this instruction said to mark it the way the two package registries are marked; those carry no marker at all and pass only because exempt() names them by hand, so following it literally would have produced a missing-marker failure. Adversarial finding AR-30 recorded it. Registering the file is split across three owners, none of them this unit: W114 adds it to the prod arm of skill_files(), W81 adds its manifest row and W82 its map entry. An earlier version of this sentence sent a reader to W114 for both halves, and W114 owns only the arms; adversarial finding AR-43 recorded that the file would have reached skill_files() and never the manifest, failing the test that compares the two.

## Acceptance criteria

§ 6.1
The registry has one row per artifact the matrix builds, each checksum matches the built artifact, the file parses with awk alone, and the marker gate reads its MODE line. It is the single declaration of what ships: no other file states a different set.

## Handoff

§ 7.1
W111 has a declaration to validate, W104 has the mapping from a host pair to one artifact rather than deriving it from a file list, and CODE-STYLE section 1b names a file that exists.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
