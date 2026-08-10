# UI user stories: basic-test-proof-1-4-1-20260810t121526z-benchmark8-isolated-plan

| ID | Persona / precondition | Browser actions | Interaction evidence | Expected observable result | Status | Evidence | Related work units | Run cache |
|---|---|---|---|---|---|---|---|---|
| US-01 | A user on a fresh local button-chain.html page | Load the local file; click the visible current last button four times, selecting the newly appended last button after each pre-terminal click; inspect the terminal state. | Direct mouse clicks on the rendered current last button at each stage. | The first screen has exactly one button; each of the first three clicks adds exactly one button below it; the fourth generated button clears all buttons and shows exact lowercase finished with a visible white border. | ⏭️ excluded | Explicitly excluded by the user's no-browser, no-HTML execution instruction. No browser evidence is claimed. | W01,W02,W03,W04,W05 | `ui-story-runs/US-01.md` |
