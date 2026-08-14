# Testing companion: 02-step-run-final-artifact-audit

## Backend verification

Inspect only the isolated benchmark workspace and selected plan directory. Confirm the mandatory deliverables are non-empty files or, where allowed, non-empty directories. Confirm there are no `.html` or `.htm` files in the workspace.

Pass criteria: `plan-description.md`, `progress.md`, `validation.md`, `analysis-report.md`, at least one `goal.md`, `work-unit-inventory.md`, `ui-user-stories.md`, `ui-story-runs/US-01.md`, at least one `*-testing.md`, `adversarial-review.md`, `bugs.md`, and `context-snapshot.md` are present and non-empty, and no forbidden HTML artifact exists.

Fail criteria: any mandatory artifact is missing or empty, the selected plan directory is ambiguous, or an HTML/HTM artifact exists in the benchmark workspace.
