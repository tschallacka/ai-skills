# Step: 02-step-monitor-continuation

## Ownership

- Goal: `10-plan-integrity-and-execution-persistence`
- Work unit: `W65`
- Type: `docs`

## Change target

- File: `benchmark/planning/README.md`
- Primary symbol or file scope: `monitor continuation contract`
- Subscope: `N/A`

## Objective

Prevent monitoring from treating an intermediate subprocess status report as completion.

## Instructions

1. Define monitor states for running, status-reported, steering, retrying, blocked, completed, accepted, tainted, rejected, and interrupted.
2. Treat a status report, partial artifact list, unchanged poll, or “I’m working” message as non-terminal. Continue bounded polling and issue an explicit next-action steering prompt or command while the subprocess remains active.
3. Before each steering action, inspect bounded process state, latest output, expected artifact state, and elapsed/retry budget. Never restart blindly or conceal a real error.
4. Stop only on explicit terminal evidence: process exit plus result, accepted/tainted/rejected archive, validated completion report, or a recorded blocker after the configured retry budget. Preserve the last output, process audit, next action, and reason.
5. Use a reusable `/tmp` monitor helper for repeated long checks, with bounded output, explicit run arguments, and a selector flag such as `1` (runner/worker), `2` (reviewers), or `3` (all in-scope processes) as required by W62.

## Acceptance criteria

- The monitor contract clearly distinguishes intermediate status from terminal completion.
- A live subprocess with a status-only message receives continued bounded steering until completion, blocker, or retry exhaustion.
- Every steering action records process/output evidence, next action, and retry count.
- The monitor selector flag narrows the process set and unsupported flags fail closed.
- Interrupted, tainted, rejected, and genuinely blocked runs preserve evidence and are not reported as successful.

## Handoff

Hand off the monitor state, last subprocess evidence, steering actions, retry budget, and terminal reason to the run archive and plan working context.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
