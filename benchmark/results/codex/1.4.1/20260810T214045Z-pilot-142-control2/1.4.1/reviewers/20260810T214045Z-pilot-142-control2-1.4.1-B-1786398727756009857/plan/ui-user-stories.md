# UI user stories: basic-test-proof-1.4.1-20260810T214045Z-pilot-142-control2-isolated-plan

| ID | Persona / precondition | Browser actions | Interaction evidence | Expected observable result | Status | Evidence | Related work units | Run cache |
|---|---|---|---|---|---|---|---|---|
| US-01 | Keyboard-and-mouse user opening the future local HTML file | Open button-chain.html in a browser, click the initial last button, then click each newly appended current last button until the fourth generated button is clicked. | Mouse clicks on the visible current last button only through the rendered page; developer-tool shortcuts are outside the accepted evidence. | After the first three generated-button clicks, one new button appears below the prior last button each time. After clicking the fourth generated button, the page shows only the exact lowercase text finished with a visible white border. | 💤 untested | — | W02,W03,W05 | `ui-story-runs/US-01.md` |
