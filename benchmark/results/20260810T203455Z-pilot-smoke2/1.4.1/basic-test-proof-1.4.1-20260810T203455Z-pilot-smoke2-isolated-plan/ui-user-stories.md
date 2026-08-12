# UI user stories: basic-test-proof-1.4.1-20260810T203455Z-pilot-smoke2-isolated-plan

| ID | Persona / precondition | Browser actions | Interaction evidence | Expected observable result | Status | Evidence | Related work units | Run cache |
|---|---|---|---|---|---|---|---|---|
| US-01 | A user opening the standalone button-chain page | Open button-chain.html; confirm one initial button; click the initial button, generated button 1, generated button 2, generated button 3, then generated button 4. | Five separate mouse clicks on the visible current last button: initial button, generated button 1, generated button 2, generated button 3, and generated button 4. | Each of the first four clicks appends exactly one button below the previous last button; clicking generated button 4 clears the page and shows exact lowercase text finished with a visible white border. | 💤 untested | — | W01,W02,W03,W04,W05 | `ui-story-runs/US-01.md` |
