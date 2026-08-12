# Testing companion: 01-step-write-analysis-report

## Backend verification

Inspect `analysis-report.md` after it is written and confirm it contains revision, start timestamp, end timestamp, elapsed time, worker result, validation result, review result, artifact/process audit, filesystem-boundary audit, session ID source/value, and token usage or explicit unavailable status.

Pass criteria: every required evidence field is present with an actual value or an explicit unavailable status, and no field invents telemetry.

Fail criteria: missing report, empty report, placeholder values, inferred token counts, or missing validation/review/audit evidence.
