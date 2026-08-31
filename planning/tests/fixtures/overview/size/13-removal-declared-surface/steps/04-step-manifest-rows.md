# Step: 04-step-manifest-rows

## Ownership

- Goal: `13-removal-declared-surface`
- Work unit: `W81`
- Type: `config`

## Change target

- File: `planning/PACKAGE-MANIFEST.tsv`
- Primary symbol or file scope: `removed renderer rows`
- Subscope: `N/A`

## Objective

§ 4.1
Remove the rows naming the renderer, its template, the serve wrapper and the runtime directory, and add the rows for the prebuilt artifacts. A manifest that lists a deleted file fails its own freshness check.

## Instructions

§ 5.1
In planning/PACKAGE-MANIFEST.tsv remove the seven rows for the renderer, the token template, the serve wrapper and the four runtime rungs, add one row per prebuilt artifact declared by the artifact matrix, and add the row for planning/binaries.tsv, which ships and which the manifest must name or the installer-manifest test fails on a prod-arm entry with no manifest row. Keep the row for the state writer, which is not being removed. Do not touch planning/PACKAGE-MAP.tsv; W82 owns it. Adversarial finding AR-43 recorded that binaries.tsv reached skill_files() and never the manifest, because W110 pointed at W114 for both halves and W114 owns only the arms.

## Acceptance criteria

§ 6.1
The manifest names no file that is absent from the tree, and names every artifact the matrix builds. The marker and manifest cross-check passes, and an install lists the artifacts among the installed files while listing none of the removed scripts.

## Handoff

§ 7.1
W82 has a manifest to reconcile the map against, which is what makes the two files comparable as a pair rather than each merely internally consistent. This unit depends on W113 because the cross-check it must satisfy, planning/tests/test-installer-manifest.sh, reads the skill_files list out of the generated install.sh and compares it against this file, failing on a difference in either direction — so the installer must already carry W17's and W114's arm changes before this file can agree with it. Two earlier versions of this handoff named the wrong file and the wrong ordering: the first cited installer/src/50-manifest.sh as the second copy the duplication cross-checks, which is where the list is written but not where the test reads it, and depended on W17 alone; adversarial finding AR-54 recorded that this left the registration split across three unordered units, reproducing the AR-14 window this handoff quotes as its own justification.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
