# Progress: untrack-generated-files

**Overall progress:** `100%  ####################  100%` ✅

| Goalname | Description | Completion status |
|---|---|---|
| 01-untrack-rjq-binary | git ls-files planning/bin is empty and the blob from e8bdaef is gone from the index; .gitignore carr... | ✅ completed |
| 02-untrack-compiled-libs | planning/scripts/plan-{core,crypt,document,progress,table}-lib.sh are gitignored and untracked; run-... | ✅ completed |
| 03-untrack-reviewer-md | planning/REVIEWER.md is gitignored and untracked; planning/tests/test-reviewer-projection.sh builds ... | ✅ completed |
| 04-untrack-portability-md | PORTABILITY.md is gitignored and untracked; planning/tests/test-portability-contract.sh regenerates ... | ✅ completed |
| 05-verify-artifact-migration | Cross-cutting proof that no consumer was orphaned: a scratch clean checkout plus the documented boot... | ✅ completed |
| 06-package-test-gates | tests/test-release-package.sh asserts tarball content against fresh builds instead of repo copies (w... | ✅ completed |
| 07-rjq-local-delivery | A documented one-command dev bootstrap builds src/rjq into the gitignored planning/bin/<triple> path... | ✅ completed |
