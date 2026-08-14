# Analysis report

## Execution identity and timing

- Revision: `1.2.0`
- Tagged source: `/tmp/20260810T121526Z-benchmark8/1.2.0/source/`
- Isolated workspace: `/tmp/20260810T121526Z-benchmark8/1.2.0/workspace`
- Plan: `basic-test-proof-1.2.0-20260810T121526Z-benchmark8-isolated-plan`
- Start timestamp: `2026-08-10 14:15:50.700084409 +0200` (session-id.txt filesystem timestamp immediately after session capture)
- End timestamp: `2026-08-10T14:37:14,748354972+02:00`
- Elapsed time: `1284` seconds (epoch-second calculation; timestamp precision retained above)
- Thread ID: `019feb99-8be0-7dd1-84e7-154b95333ee7`
- Thread ID source: `CODEX_THREAD_ID`, written immediately to workspace `session-id.txt`.

## Worker result

`PASS` for the planning-only proof. The plan was created with two goals and thirteen atomic work units. No HTML/HTM was created, opened, inspected, served, or tested. No browser, server, driver, or other execution tooling was started.

## Validation result

- Final tagged validator: `/tmp/20260810T121526Z-benchmark8/1.2.0/source/planning/scripts/validate-plan.sh`
- Final exit code: `0`
- Result: `Plan validation passed: 13 work units across 2 goals.`
- Complete command output is preserved in [`validation.md`](validation.md).

## Review result

- Fresh secondary adversarial review: `✅ approved`.
- Review artifact: [`adversarial-review.md`](adversarial-review.md).
- Review rounds: five fresh secondary reviews were used; earlier findings were resolved before the approved review.

## Artifact and process audit

- Mandatory artifact audit: `PASS`; all required files are non-empty, including the plan description, both goal/progress trackers, inventory, UI story, `ui-story-runs/US-01.md`, testing companions, approved review, bug register, context snapshot, validation report, and this report.
- Plan file count at final audit: `40` non-empty or substantive files under the isolated plan directory.
- Workspace HTML/HTM audit: `PASS`; zero `.html`, `.htm`, `.HTML`, or `.HTM` files found anywhere in the isolated benchmark workspace.
- Matching process audit: `PASS`; no matching browser, server, driver, or related process was present after the worker actions.
- UI execution audit: intentionally untested; US-01 remains `💤 untested` because the benchmark explicitly forbids creating or testing HTML. This is not claimed as a browser pass or user-approved exclusion.

## Token and telemetry evidence

- Telemetry source: local Codex session JSONL and runner-owned SQLite state under `/home/mdibbets/.codex/`, matched by the UUID in `session-id.txt` rather than by recency.
- Final recorded cumulative usage: `4,806,402` total tokens (`4,768,482` input, `37,920` output, including `4,637,184` cached input and `4,223` reasoning output) in the matched worker session record.
- SQLite telemetry was preserved by the runner; no telemetry database was modified.

## Process and artifact conclusion

The isolated proof produced the expected plan output and runner metadata only. The durable plan is structurally valid and review-approved, while implementation and browser acceptance remain intentionally incomplete for this planning-only benchmark.
