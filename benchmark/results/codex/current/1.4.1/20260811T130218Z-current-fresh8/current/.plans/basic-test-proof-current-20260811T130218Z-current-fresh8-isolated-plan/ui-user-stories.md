# UI user stories: basic-test-proof-current-20260811T130218Z-current-fresh8-isolated-plan

| ID | Persona / precondition | Browser actions | Interaction evidence | Expected observable result | Status | Evidence | Related work units | Run cache |
|---|---|---|---|---|---|---|---|---|
| US-01 | Future user opening the completed standalone file from a fresh browser context | Click the initial button, click the first generated last button, click the second generated last button, click the third generated last button, then click the fourth generated last button. | Five direct mouse clicks or taps on visible button controls; each click targets the current last button in visual order. | After the fourth generated button is clicked, the document content is cleared and the only completion state shows exact text finished with a visible white border against a contrasting state. | 💤 untested | — | W01,W02,W03,W04,W05,W06 | `ui-story-runs/US-01.md` |
