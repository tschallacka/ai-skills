# Benchmark result: 1.4.1

## Execution

- Revision: `1.4.1`
- Source report: `basic-test-proof-1.4.1-isolated-plan-rerun/1.4.1-analyze.md`
- Start: `2026-08-08T00:35:57+02:00`
- End: `2026-08-08T00:42:39+02:00`
- Elapsed: **402 seconds**
- Mode: fresh isolated planning-only worker run
- Work units: **7 across 2 goals**
- Thread: `019fde5e-6fa1-7940-a1a5-4a55ce4aebd3`

## Verification

- Plan validation: passed
- Context audit: passed
- Context benchmark: **76% small, 92% medium, 91% coupled reduction**
- Adversarial review: approved through a sequential pass; independent subagent
  review was unavailable because the run prohibited subagents
- HTML/browser/server execution: deferred; no HTML artifact created
- CLI-reported usage: **2,003,919 tokens**
- SQLite telemetry: unavailable because this run used ephemeral mode

## Result

The 1.4.1 run used the fewest reported tokens and completed 17 seconds faster
than 1.3.1. Its token figure is not directly equivalent to the 1.3.1 and
1.4.0 SQLite totals because it came from the CLI turn summary rather than
persisted SQLite usage records.
