# Step: 09-step-regenerate-installer

## Ownership

- Goal: `16-toolchain-and-packaging`
- Work unit: `W113`
- Type: `generated`

## Change target

- File: `install.sh`
- Primary symbol or file scope: `regenerated installer`
- Subscope: `N/A`

## Objective

§ 4.1
Regenerate and commit install.sh after the declarations it is generated from have changed. It is a tracked generated file whose freshness gate diffs it against its source, so leaving it uncommitted reddens two gates while no unit owns the file. This is the inseparable generated-file exception: build.sh writes the whole file in one pass, so individual review is impossible.

## Instructions

§ 5.1
Run bash installer/build.sh and commit the resulting install.sh. The generator is the only writer: never hand-edit the file. It concatenates every installer/src/NN-name.sh and reads installer/tools.tsv, each skill's requires.tsv and installer/probe-allowlist.tsv, so confirm the diff contains only what those inputs' changes imply — the four removed runtime rows from W80's requires.tsv edit, and the installer/src changes from W17, W114, W104 and W18, all four of which edit that directory. An unexplained hunk means an input changed that no unit owns, which is a finding rather than something to commit. planning/binaries.tsv is not an input to the generator and produces no hunk here. Two earlier versions of this instruction were wrong in opposite directions: the first listed binaries.tsv as producing a hunk while omitting W114 and W104, whose hunks are real (AR-48), and the second enumerated three installer/src editors when there are four, leaving W18's hunk ownerless (AR-52).

## Acceptance criteria

§ 6.1
installer/build.sh --check passes against the committed install.sh, the installer freshness gate passes, and every hunk in the diff traces to one of the five inputs named in the instructions. Regenerating a second time produces no further diff, which is what proves the committed copy is the generator's fixed point rather than a hand-edited approximation of it. The manifest gate is deliberately not asserted here and is W81's and W82's: it compares PACKAGE-MANIFEST.tsv against the map's installable rows and against the skill_files list read out of this regenerated install.sh, so it cannot be green until those two files have caught up, and both depend on this unit. An earlier version of this criterion asserted both gates; adversarial finding AR-54 moved W81 downstream of this unit and AR-58 recorded that the criterion was never re-read, so it asserted a gate that is red by construction at exactly this point.

## Handoff

§ 7.1
W48 can assert that the generated installer matches its source against a tree where every unit that edits installer/src has already run and some unit has actually regenerated the file. Adversarial finding AR-28 recorded that W80 and W17 each instructed the regeneration inside a step whose atomicity check certifies that no other file changes, while no row owned install.sh at all; adversarial finding AR-42 recorded that the unit created to fix that was not downstream of W114 and W104, so a permitted order left install.sh stale and W48 red.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
