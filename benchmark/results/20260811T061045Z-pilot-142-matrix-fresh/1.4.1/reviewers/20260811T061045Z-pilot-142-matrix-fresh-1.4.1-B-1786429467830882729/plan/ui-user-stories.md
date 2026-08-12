# UI user stories: basic-test-proof-1.4.1-20260811T061045Z-pilot-142-matrix-fresh-isolated-plan

| ID | Persona / precondition | Browser actions | Interaction evidence | Expected observable result | Status | Evidence | Related work units | Run cache |
|---|---|---|---|---|---|---|---|---|
| US-01 | Manual browser verifier | Open button-chain.html, click the current last button five times in sequence: initial, generated 1, generated 2, generated 3, then generated 4 | Mouse clicks on the visible current last button after each append, including the generated 4 button as the terminal click | After clicks one through four exactly one new button appears below the previous last button, creating generated buttons 1 through 4; after click five on generated 4 the document is cleared and shows only finished with a visible white border. | 💤 untested | — | W01,W02,W03,W04,W05,W07 | `ui-story-runs/US-01.md` |
