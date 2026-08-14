# Testing companion: 01-step-create-initial-markup

## Browser verification

This step is verified downstream by `W05` / `US-01` after future implementation. On a fresh browser load of `button-chain.html`, record that exactly one button is visible before any click and that no generated button or `finished` completion state is visible.

Pass criteria: the initial body contains exactly one visible button and no completion message. Fail criteria: zero buttons, more than one button, a pre-rendered generated button, or any initial `finished` text.

## Automated tests

No separate automated test file is planned for this basic standalone HTML proof. The required proof is the direct browser story in `02-verify-button-chain-flow/steps/01-step-run-ui-story.md`.
