# Testing companion: 02-step-browser-story

## Browser verification

- Target work unit: `W05`
- Story: `US-01`
- Future target: the implemented `button-chain.html` opened as a local file URL or approved local route.
- Direct interaction sequence:
  1. Click the initial button.
  2. Click the new current last button.
  3. Click the new current last button.
  4. Click the new current last button so the fourth generated button is visible.
  5. Click the fourth generated button.
- Pass criteria:
  - Each of the first four clicks appends exactly one button below the prior last button.
  - No click on an earlier button is needed or used to pass the story.
  - The final click clears prior document content.
  - The only completion message text is the exact lowercase text `finished`.
  - The completion message has a visible white border.

## Result for this planning proof

- Status: not run.
- Evidence: browser, server, driver, and HTML execution tooling are forbidden during this planning-only proof.
- Completion requirement for future executor: update `ui-user-stories.md` and `ui-story-runs/US-01.md` with browser evidence before marking W05 complete.
