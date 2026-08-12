# UI user stories: basic-test-proof-1.4.1-20260810T210953Z-pilot-142-control-isolated-plan

| ID | Persona / precondition | Browser actions | Interaction evidence | Expected observable result | Status | Evidence | Related work units | Run cache |
|---|---|---|---|---|---|---|---|---|
| US-01 | User opening the completed local button-chain.html file in a browser | Click the visible initial button, then click each newly appended last button until the fourth generated button is clicked. | Mouse clicks on the visible current-last button for the initial, first generated, second generated, third generated, and fourth generated controls. | The first four clicks each append exactly one button below the current last button; clicking the fourth generated button clears the document and shows exact text finished inside a visible white border. | 💤 untested | — | W01,W02,W03,W06,W05 | `ui-story-runs/US-01.md` |
