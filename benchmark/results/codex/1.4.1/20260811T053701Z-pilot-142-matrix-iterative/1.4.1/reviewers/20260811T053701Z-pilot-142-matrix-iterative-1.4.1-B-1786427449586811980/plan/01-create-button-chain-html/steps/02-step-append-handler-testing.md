# Testing companion: 02-step-append-handler

## Static verification

After W02 is executed, inspect `button-chain.html` source and confirm the click handler:

- Acts only when the clicked button is the current last button.
- Appends exactly one button per valid non-terminal click.
- Tracks generated buttons separately from the initial button.
- Appends generated button 4 before it can be clicked as the terminal trigger.
- Clears the document when generated button 4 is clicked.
- Emits exact lowercase text `finished` for the terminal state.

## Browser verification

Covered downstream by `W05` / `US-01` through five direct rendered-button clicks. Browser evidence is required before future execution can be marked complete.
