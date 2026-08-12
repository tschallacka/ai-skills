# Testing companion: 03-step-style-button-stack

## Static verification

After future implementation, inspect `button-chain.html` for styling scoped to `.chain-button`. The style must make buttons render vertically so appended generated buttons appear below the previous current-last button.

## Browser verification

Covered downstream by UI story `US-01`, which observes each appended generated button below the prior last button during direct mouse-click interaction.

## Pass/fail criteria

Pass when the chain button layout creates a vertical stack without affecting the terminal `.completion-state` border. Fail if generated buttons can render beside the previous current-last button or if layout depends on fragile browser defaults.
