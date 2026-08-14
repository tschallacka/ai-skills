# Testing companion: 01-step-us-01-validation

## Browser verification
- Use the cache `ui-story-runs/US-01.md` from a fresh browser context and the future local file route for `button-chain.html`.
- Execute the five cached mouse clicks in order, waiting for the next rendered current-last-button target after each click.
- Pass only when the initial one-button state, four exact one-at-a-time appends creating generated buttons 1–4, pressing generated button 4 on click five, exact lowercase `finished`, and visible white border are all observed and recorded in `ui-user-stories.md` and the run cache.
- Fail by recording the actual evidence, setting the story to `🐛 bug found` when the behavior is unexpected, and adding investigation/fix traceability before any later implementation work.
- Isolated-proof status: intentionally `💤 untested`; no browser, server, driver, or HTML execution was performed.
