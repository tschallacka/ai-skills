# Testing companion: 03-step-finished-style

## Static verification

After W03 is executed, inspect `button-chain.html` source and confirm `.completion-message` defines a white border with visible width and style.

Pass criteria:

- The border color is white.
- The terminal element or body state keeps the border visible against its background.
- Styling does not alter the required lowercase text `finished`.

## Browser verification

Covered downstream by `W05` / `US-01`, which must visibly confirm the white border after the terminal click.

