# Testing companion: 03-step-behavior

## Browser verification

Run US-01 from a fresh page. Use only mouse clicks on the currently visible last button. After clicks 1, 2, 3, and 4, count the rendered buttons and confirm counts 2, 3, 4, and 5 respectively, with generated buttons 1–4 exactly one immediate next button below the previous last button each time. Press generated button 4 on click 5 and confirm the root is removed, zero buttons remain, and the document is cleared to exactly one visible terminal element with text `finished` on a contrasting background and an explicit 1px solid white border. Any extra button, missing append, wrong click target, off-by-one terminal state, or premature clearing fails the step.

## Automated tests

No separate automated test is planned for the future inline handler. The bounded browser flow is the discriminating proof and is owned by W04.
