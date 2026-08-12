# Testing companion: 01-step-initial-markup

## Static verification

After W01 is executed in the future implementation session, inspect `button-chain.html` source without opening or running it.

Pass criteria:

- `button-chain.html` exists.
- Source markup contains exactly one initial `<button>` in `#button-chain-root` before runtime interaction.
- No generated buttons are hard-coded in source markup.
- The initial button has user-visible text suitable for direct browser targeting.

## Browser verification

Covered downstream by `W05` / `US-01`. Do not run browser tooling during this planning-only proof.
