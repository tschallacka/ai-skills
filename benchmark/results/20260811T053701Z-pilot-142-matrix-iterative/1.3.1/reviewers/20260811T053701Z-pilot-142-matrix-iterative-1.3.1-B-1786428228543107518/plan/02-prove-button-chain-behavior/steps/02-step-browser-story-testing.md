# Testing companion: 02-step-browser-story

## Browser verification
- Future executor opens `button-chain.html` in a browser and follows `ui-story-runs/US-01.md` using normal mouse clicks.
- Pass criteria: clicks one through four append exactly one button below the previous last button; click five on the fourth generated button clears the document and shows only `finished` with a visible white border.

## Evidence to record
- URL or file path opened.
- Button count after each click.
- Final visible text and border evidence.
- Updated `ui-user-stories.md`, `ui-story-runs/US-01.md`, and `bugs.md` if a bug is found.

## Deferred execution note
- This benchmark run is planning-only, so no browser, server, or driver was started.
