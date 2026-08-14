# Bug register

## Current result

No bugs were observed or reproduced because this benchmark is planning-only and explicitly forbids creating, opening, serving, or testing HTML. The required future browser story remains `US-01` with status `💤 untested`; its cache records the exact interaction that will establish bug evidence during execution.

## Recovery policy

If the future US-01 run fails, stop the affected batch at the first discrepancy. Change US-01 to `🐛 bug found` and record the exact reproduction, actual result, browser evidence, severity, story ID, and failed cache order in `bugs.md`. Before any fix starts, add uniquely numbered atomic `NN-investigate-<bug-slug>` and dependent `NN-fix-<bug-slug>` goals, named in the bug row, plus their test work units and the US-01 retest story. Update plan-description.md, work-unit-inventory.md, ui-user-stories.md, both progress trackers, and the context snapshot to show the new scope. Resolve the bug only after the fix and direct browser retest pass; rerun US-01 and every story depending on changed units. For a severe blocker (journey blocked, data loss, authorization/privacy risk, page load failure, or unreliable downstream results), pause the current goal, prioritize investigation and fix, and restart story validation from US-01 after the fix. Never weaken the story to obtain a pass.
