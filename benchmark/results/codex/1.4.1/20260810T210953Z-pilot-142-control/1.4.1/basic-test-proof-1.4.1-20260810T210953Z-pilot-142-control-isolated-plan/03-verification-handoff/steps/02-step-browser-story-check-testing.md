# Testing companion: 02-step-browser-story-check

## Browser verification

Use the cached sequence in `ui-story-runs/US-01.md`. Open the future local `button-chain.html`, click the current last visible button five times total: initial button, generated button 1, generated button 2, generated button 3, and generated button 4.

## Pass/fail criteria

Pass only if the first four accepted clicks each append exactly one button below the previous last button, and the fifth click on generated button 4 clears the document to exact lowercase text `finished` inside a visible white border. Fail if the story requires non-UI shortcuts or if any generated count, clear, text, or border expectation is wrong.

## Bug loop

On failure, do not fix inside this verification step. Update `ui-user-stories.md` so `US-01` is `🐛 bug found`, add a bug row with reproduction, actual result, evidence, severity, investigation goal, fix goal, and retest story, then add new investigation and fix goals plus their work units before resuming execution. Rerun `US-01` after the fix and keep the original bug evidence.
