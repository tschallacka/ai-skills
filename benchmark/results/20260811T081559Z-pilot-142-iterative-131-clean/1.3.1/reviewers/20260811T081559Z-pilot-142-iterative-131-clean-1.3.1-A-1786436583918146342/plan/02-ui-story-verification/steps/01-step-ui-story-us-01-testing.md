# Testing companion: 01-step-ui-story-us-01

## Browser verification

Run only after the future executor has created `button-chain.html`.
Open the file through the browser as a local file URL. Do not use a console
command, injected event, storage edit, direct API request, or DOM mutation to
advance the state.

1. Confirm the initial page shows exactly one button and no `finished` text.
2. Mouse-click the current last button. Confirm exactly one generated button
   appears below it.
3. Mouse-click generated button 1. Confirm exactly one generated button 2
   appears below it.
4. Mouse-click generated button 2. Confirm exactly one generated button 3
   appears below it.
5. Mouse-click generated button 3. Confirm exactly one generated button 4
   appears below it.
6. Mouse-click generated button 4. Confirm the document is cleared, no buttons
   remain, and the only visible completion text is exactly `finished` with a
   visible white border.

## Pass/fail criteria

Pass only when the complete direct-click sequence matches `US-01` and the run
cache records observed waits, evidence, and `✅ passed`. Any extra appended
button, response to an older non-last button, missing clear, uppercase or extra
completion text, or non-visible border fails and requires a `bugs.md` entry.
