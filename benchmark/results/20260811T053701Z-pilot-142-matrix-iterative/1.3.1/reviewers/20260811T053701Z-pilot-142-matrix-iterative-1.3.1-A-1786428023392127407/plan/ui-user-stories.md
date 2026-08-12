# UI user stories: basic-test-proof-1.3.1-20260811T053701Z-pilot-142-matrix-iterative-isolated-plan

| ID | Persona / precondition | Browser actions | Interaction evidence | Expected observable result | Status | Evidence | Related work units | Run cache |
|---|---|---|---|---|---|---|---|---|
| US-01 | User opens future local button-chain.html with one visible initial button | Open button-chain.html, then click the current last visible button five times: initial, generated one, generated two, generated three, generated four | Mouse click on the current last button for each step of the chain | The first four clicks each append exactly one button below the previous last button, and the fifth click clears the document to exact text finished with a visible white border | 💤 untested | — | W01,W02,W03,W04,W06 | `ui-story-runs/US-01.md` |
