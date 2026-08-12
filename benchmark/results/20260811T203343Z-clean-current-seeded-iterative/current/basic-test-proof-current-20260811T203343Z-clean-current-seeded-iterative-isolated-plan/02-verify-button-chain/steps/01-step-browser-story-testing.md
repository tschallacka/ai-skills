# Verification: 01-step-browser-story

## Browser verification

Open `button-chain.html` in a browser from a fresh load state. Follow `ui-story-runs/US-01.md` exactly: click the initial button, generated button 1, generated button 2, generated button 3, and generated button 4. After each click, record the observed readiness result in the run cache.

Pass only if the initial button and generated buttons 1 through 4 are clicked in order with real UI input, each pre-finish click appends exactly one button below the previous last button, and the final view contains exactly `finished` with a visible white border. Any console command, injected event, storage edit, direct API call, or internal function call invalidates the story evidence.
