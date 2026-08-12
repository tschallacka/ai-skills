# Testing companion: 01-step-create-root-markup

## Static verification

After the future implementation creates `button-chain.html`, inspect only that file. Confirm the initial document contains exactly one visible button, a single `#button-chain-root` subtree, and no pre-rendered generated buttons or `finished` completion state.

## Browser verification

Covered downstream by `03-verification-handoff/steps/02-step-browser-story-check.md` and UI story `US-01`, which starts from the initial one-button state.

## Pass/fail criteria

Pass when the initial state is one button only and the root target is stable for later click handling. Fail if extra buttons, terminal text, missing root markup, or unrelated files are introduced.
