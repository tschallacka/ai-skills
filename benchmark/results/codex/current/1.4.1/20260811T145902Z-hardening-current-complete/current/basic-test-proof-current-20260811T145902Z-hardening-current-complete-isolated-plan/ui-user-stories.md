# UI user stories: basic-test-proof-current-20260811T145902Z-hardening-current-complete-isolated-plan

| ID | Persona / precondition | Browser actions | Interaction evidence | Expected observable result | Status | Evidence | Related work units | Run cache |
|---|---|---|---|---|---|---|---|---|
| US-01 | User opens the future standalone button-chain.html from a clean initial load | Click the visible current last button repeatedly, using the newly appended button as the next click target | Direct mouse click on the initial button, then direct mouse clicks on generated buttons 1, 2, 3, and 4 as each becomes current last | Exactly one button is appended below the current last button for each non-terminal click; clicking the fourth generated button clears the document and shows exact lowercase text finished with a visible white border | 💤 untested | — | W05 | `ui-story-runs/US-01.md` |
