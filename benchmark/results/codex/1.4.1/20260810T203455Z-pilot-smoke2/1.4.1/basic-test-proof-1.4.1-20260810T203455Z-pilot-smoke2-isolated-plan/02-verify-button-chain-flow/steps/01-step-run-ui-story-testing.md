# Testing companion: 01-step-run-ui-story

## Browser verification

Run `US-01` from `ui-story-runs/US-01.md` after future implementation. Open `button-chain.html`, confirm the fresh initial state, click the initial button, generated button 1, generated button 2, generated button 3, and generated button 4 as separate direct UI inputs. Record the route or local file target, direct click sequence, intermediate button counts after every click, final visible text, and border evidence.

Pass criteria: the first eligible clicks append exactly one button below the current last button, clicking generated button four clears the document, the only completion text is exact lowercase `finished`, and the completion state has a visible white border.

Fail criteria: any append count mismatch, wrong insertion location, terminal state on the wrong button, retained old controls after completion, incorrect final text, or missing/non-white border. On failure, update `bugs.md` and add investigation/fix/retest plan work before acceptance.

## Automated tests

No automated command is required for this proof. The future executor may add one only by revising the plan with a new atomic test work unit.
