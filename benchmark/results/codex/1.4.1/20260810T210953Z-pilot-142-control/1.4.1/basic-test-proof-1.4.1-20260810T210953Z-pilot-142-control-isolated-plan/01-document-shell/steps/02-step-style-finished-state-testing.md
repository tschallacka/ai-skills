# Testing companion: 02-step-style-finished-state

## Static verification

After future implementation, inspect `button-chain.html` for a `.completion-state` selector or equivalent scoped rule that creates a visible white border around the terminal state.

## Browser verification

Covered downstream by UI story `US-01`, which verifies that the visible terminal `finished` state has a white border after the fourth generated button is clicked.

## Pass/fail criteria

Pass when the completion state has an explicit white border and no style change alters the initial one-button layout. Fail if the border is absent, not white, or not applied to the terminal `finished` state.
