# Step: 05-step-package-map

## Ownership

- Goal: `13-removal-declared-surface`
- Work unit: `W82`
- Type: `config`

## Change target

- File: `planning/PACKAGE-MAP.tsv`
- Primary symbol or file scope: `removed renderer entries`
- Subscope: `N/A`

## Objective

§ 4.1
Remove the map entries for the same deleted files and record the artifacts in their place, so the map and the manifest agree on what ships.

## Instructions

§ 5.1
In planning/PACKAGE-MAP.tsv remove the same seven source-to-destination entries W81 removed from the manifest, add the artifact entries in their place with the destination each artifact installs to, and add the entry for planning/binaries.tsv so the map and the manifest name the same set. Keep the header row and the state writer entry. The map is edited to agree with the manifest, not independently of it, and the installer-manifest test requires the manifest to equal the map's installable rows.

## Acceptance criteria

§ 6.1
The map and the manifest name the same set of files, in both directions: no source appears in one and not the other. A reconciliation of the two reports no difference, and an install driven from the map places every artifact at the destination the manifest declares.

## Handoff

§ 7.1
W83 and W84 can describe how the overview is rendered and served knowing the shipped file set behind that description is settled, and goal 09 has a manifest and a map that agree, which is the pair its verification reads.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
