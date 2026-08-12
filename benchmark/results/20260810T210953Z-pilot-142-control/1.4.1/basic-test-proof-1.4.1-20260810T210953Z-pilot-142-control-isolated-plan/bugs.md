# UI bugs: basic-test-proof-1.4.1-20260810T210953Z-pilot-142-control-isolated-plan

| ID | Story | Severity | Reproduction/evidence | Investigation goal | Fix goal | Retest story | Status |
|---|---|---|---|---|---|---|---|
| BUG-00 | N/A | N/A | No browser story was executed during this planning-only proof, by benchmark instruction. Future story failures must be added here with reproduction evidence before fixes proceed. | N/A | N/A | N/A | No bugs observed in planning-only run |

## Required failure workflow

If `US-01` fails in future execution, update the story status to `🐛 bug found`, record the direct browser reproduction and actual result, classify severity, create both `NN-investigate-<bug-slug>` and `NN-fix-<bug-slug>` goals, add their atomic work units to the inventory, update plan and goal progress trackers, then retest `US-01` and any dependent story before marking verification complete.
