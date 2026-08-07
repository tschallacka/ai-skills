# Basic test proof plan

## Purpose

Run a planning-only proof against two skill revisions and record comparable
execution and token-cost evidence.

## Test task

The worker must create a durable implementation plan for an HTML task. The
planned task is: create an HTML file containing one button; pressing the
current last button appends one new button below it; pressing the fourth
generated button clears the document and prints `finished` with a white
border.

The worker must not create, edit, or test the HTML file during this proof. The
HTML behavior is acceptance criteria inside the plan only. This isolates the
planning skill's discovery, decomposition, review, adversarial review, and
handoff behavior.

The worker must use the planning skill, shell commands for file operations, and
sequential execution. It must close any process it starts before handing off.

## Acceptance proof

- The planning artifacts are resumable and identify the exact future HTML
  output and behavior.
- The plan includes implementation, verification, acceptance, handoff, and
  review evidence for the button-chain behavior.
- The report records revision, start time, end time, elapsed time, worker
  result, and token-cost evidence or an explicit unavailable result.
- The same task is run once with the current skill (`1.4.0`) and once with the
  `v1.3.0` skill, without parallel workers.

## Reusable execution sequence

1. Record an ISO-8601 start timestamp.
2. Run one worker with the selected skill revision and this task.
3. Inspect the worker's durable plan and review artifacts; confirm no HTML
   implementation artifact was created.
4. Close the worker and terminate any test process it left behind.
5. Record an ISO-8601 end timestamp and calculate elapsed seconds.
6. Add the result to `1.4.0-analyze.md`, including history token evidence.

## Safety boundary

Do not spawn parallel workers or leave browser, server, or shell processes
running. The worker may expand the plan and perform its required sequential
review/adversarial cycle. If token cost cannot be derived from local history,
record that fact instead of inventing a number.
