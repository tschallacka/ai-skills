# UI user stories: basic-test-proof-1.3.1-20260810T121526Z-benchmark8-isolated-plan

| ID | Persona / precondition | Browser actions | Interaction evidence | Expected observable result | Status | Evidence | Related work units | Run cache |
|---|---|---|---|---|---|---|---|---|
| US-01 | A visitor opening the future local button-chain.html file in a fresh browser context | Open the future local file route, confirm one button, click the current last button four times to create generated buttons 1–4, click generated button 4 once, and observe the completion state. | Mouse click on the rendered current last button for each of five ordered activations. | The initial view has one button; clicks 1–4 each add exactly one button below the prior last button, producing generated buttons 1–4; click 5 presses generated button 4, clears the document, and leaves exact lowercase text finished with a visible white border. | 💤 untested | — | W01,W02,W03,W04,W05 | `ui-story-runs/US-01.md` |
