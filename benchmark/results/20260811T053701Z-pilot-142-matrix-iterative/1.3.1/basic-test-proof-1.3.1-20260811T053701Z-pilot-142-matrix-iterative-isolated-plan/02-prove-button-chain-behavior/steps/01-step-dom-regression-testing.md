# Testing companion: 01-step-dom-regression

## Automated test command
- Future executor selects the repository-local lightweight command appropriate for `button-chain.html`; if no project harness exists, document and run a bounded local DOM script dedicated to this file.

## Required assertions
- Initial state has exactly one button.
- Each current-last-button click before completion appends exactly one button.
- Stale non-last buttons do not append.
- The fourth generated button clears the document.
- Completion text is exactly `finished`.
- Completion style exposes a visible white border contract.

## Deferred execution note
- This benchmark run is planning-only, so no command was run.
