# Basic test proof plan

## Purpose

Run a minimal end-to-end planning-skill proof against two skill revisions and
record comparable execution and token-cost evidence.

## Test task

Have the worker create an HTML file containing one button. Pressing the current
last button must append one new button below it. The fourth generated button
must clear the document and print `finished` with a white border.

The worker must use the planning skill, shell commands for file operations, and
sequential execution. It must close any process it starts before handing off.

## Acceptance proof

- The planning artifacts are resumable and identify the exact HTML output.
- The HTML has the required button-chain behavior and fourth-button terminal
  state.
- The report records revision, start time, end time, elapsed time, worker
  result, and token-cost evidence or an explicit unavailable result.
- The same task is run once with the current skill (`1.4.0`) and once with the
  `v1.3.0` skill, without parallel workers.

## Reusable execution sequence

1. Record an ISO-8601 start timestamp.
2. Run one worker with the selected skill revision and this task.
3. Inspect the worker output and HTML artifact.
4. Close the worker and terminate any test process it left behind.
5. Record an ISO-8601 end timestamp and calculate elapsed seconds.
6. Add the result to `1.4.0-analyze.md`, including history token evidence.

## Safety boundary

Do not spawn parallel workers or leave browser, server, or shell processes
running. If token cost cannot be derived from local history, record that fact
instead of inventing a number.
