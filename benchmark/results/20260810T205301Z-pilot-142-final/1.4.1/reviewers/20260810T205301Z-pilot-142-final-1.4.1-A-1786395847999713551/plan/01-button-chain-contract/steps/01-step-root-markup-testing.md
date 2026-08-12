# Testing companion: 01-step-root-markup

## Planned verification

After the future executor creates `button-chain.html`, inspect the rendered browser state through normal page load only. Confirm exactly one visible initial button is present in `#button-chain-root`, no generated buttons exist yet, and the word `finished` is absent.

## Pass criteria

Pass only when the initial UI has one button and no completion state. Do not run this during the planning-only proof.
