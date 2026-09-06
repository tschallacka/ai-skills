# Progress: 01-untrack-rjq-binary

**Progress:** `100%  ####################  100%` ✅

| Goalname | Stepname | Description | Completion status |
|---|---|---|---|
| 01-untrack-rjq-binary | 01-step-ignore-bin-dir | Correct the stale tracked-artifact comment and ignore the triple-shaped per-target artifact director... | ✅ completed |
| 01-untrack-rjq-binary | 02-step-manifest-binary-presence | A listed bin/<triple> row resolves as optional-on-disk: when the file exists it must be a regular ex... | ✅ completed |
| 01-untrack-rjq-binary | 03-step-bundled-rjq-notice | Prepend only when the triple dir and binary exist; when absent print a one-line notice naming the re... | ✅ completed |
| 01-untrack-rjq-binary | 04-step-tracked-binary-guard | Add the guard that fails when git ls-files names any file under planning/bin or chat/bin, so a re-co... | ✅ completed |
| 01-untrack-rjq-binary | 05-step-maintainer-doc | State that per-target artifacts are CI-delivered and untracked, and that a local planning/bin/<tripl... | ✅ completed |
| 01-untrack-rjq-binary | 06-step-verify-untrack | On the working tree after W01-W05: git ls-files planning/bin is empty; a scratch install from a tree... | ✅ completed |
| 01-untrack-rjq-binary | 07-step-install-skip-missing-binary | When a platform-conditional bundled-binary row has no file on disk, skip the copy and print the same... | ✅ completed |
