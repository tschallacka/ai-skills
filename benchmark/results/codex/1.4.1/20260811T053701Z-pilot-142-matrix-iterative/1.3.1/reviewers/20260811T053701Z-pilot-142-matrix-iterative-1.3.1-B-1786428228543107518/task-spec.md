# Basic test proof plan

## Purpose

Run a planning-only proof against planning skill revisions and record
comparable execution and token-cost evidence.

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

## Acceptance proof

- The planning artifacts are resumable and identify the exact future HTML
  output and behavior.
- The plan includes implementation, verification, acceptance, handoff, and
  review evidence for the button-chain behavior.
- The report records revision, start time, end time, elapsed time, worker
  result, and token-cost evidence or an explicit unavailable result.

## Safety boundary

Do not leave browser, server, or shell processes running. If token cost cannot
be derived from local history, record that fact instead of inventing a number.
