# Testing companion: 02-step-browser-story

## Browser verification

Run `US-01` from `ui-story-runs/US-01.md` after W01-W04 pass.

Pass criteria:

- Start from a fresh render of implemented `button-chain.html`.
- Click the current last visible button five times using normal browser input.
- After clicking the initial button and generated buttons 1 through 3, exactly one new button appears below the previous last button.
- On the generated button 4 click, all prior document content is cleared.
- The remaining visible completion state is exact lowercase text `finished`.
- The `finished` text has a visible white border.

Do not use console evaluation, injected events, storage mutation, direct API calls, or internal function calls as evidence.
