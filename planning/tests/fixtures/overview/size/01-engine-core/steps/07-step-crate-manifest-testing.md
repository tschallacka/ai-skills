# Verification: 07-step-crate-manifest

## Automated tests

§ 2.1
Build the crate from a clean checkout and confirm it compiles with no network access and no dependencies resolved. Confirm the release profile is the one declared and that cargo reports zero dependencies; a crate that acquires one has broken the property this plan depends on. Run the marker gate and confirm it reads this file's marker pair, then move the pair below a comment block and confirm the gate reports the file as unmarked, which is the failure the head-of-file window creates.