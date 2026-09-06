# Progress: 02-untrack-compiled-libs

**Progress:** `100%  ####################  100%` ✅

| Goalname | Stepname | Description | Completion status |
|---|---|---|---|
| 02-untrack-compiled-libs | 01-step-ignore-compiled-libs | Add a compiled-libraries section ignoring planning/scripts/plan-core-lib.sh, plan-crypt-lib.sh, plan... | ✅ completed |
| 02-untrack-compiled-libs | 02-step-run-tests-ensure-built | Before suite discovery, build-if-missing: when any of the five compiled libs is absent run scripts/b... | ✅ completed |
| 02-untrack-compiled-libs | 03-step-libs-test-build-and-verify | Rewrite from compare-against-committed to build-and-verify: byte-compare two fresh prod builds for d... | ✅ completed |
| 02-untrack-compiled-libs | 04-step-manifest-libs-build-first | The five lib rows keep requiring presence on disk, but the test builds them first when missing, so p... | ✅ completed |
| 02-untrack-compiled-libs | 05-step-prepack-bootstrap | Prepend the generator bootstrap to prepack: run planning/scripts/build-plan-libs.sh and planning/scr... | ✅ completed |
| 02-untrack-compiled-libs | 06-step-release-build-libs | Build-if-missing the five compiled libs before the listed-file hard error, so a release build from a... | ✅ completed |
| 02-untrack-compiled-libs | 07-step-capsule-build-libs | Build the five libs into the capsule after copying planning/scripts/, on both the live-tree and git-... | ✅ completed |
| 02-untrack-compiled-libs | 08-step-agents-testing-doc | Document the bootstrap: a clean checkout gets built artifacts via the run-tests bootstrap or prepack... | ✅ completed |
| 02-untrack-compiled-libs | 09-step-development-release-doc | State that npm pack assembles generated artifacts through prepack, and that no generated file is com... | ✅ completed |
| 02-untrack-compiled-libs | 10-step-verify-clean-suite | In a scratch clean checkout (git archive of HEAD into TMPDIR): run the documented bootstrap, then ./... | ✅ completed |
