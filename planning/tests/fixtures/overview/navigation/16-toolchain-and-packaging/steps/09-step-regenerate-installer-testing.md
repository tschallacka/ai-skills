# Verification: 09-step-regenerate-installer

## Automated tests

§ 2.1
Run installer/build.sh --check and record the result. Then regenerate, and diff the result against the committed copy hunk by hunk, attributing each to the unit whose input produced it — the four installer/src editors W17, W114, W104 and W18, or W80's requires.tsv edit. Record any hunk that cannot be attributed to one of those five. Regenerate once more and confirm the second run produces no diff at all. Run the installer freshness gate and record it. Do not run the manifest gate here and do not treat its result as this unit's: it compares the manifest and the map against the list in this regenerated file, and both are corrected by later units in the removal goal, so it is red by construction at this point and green only after they run.