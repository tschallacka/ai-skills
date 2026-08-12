# UI user stories: basic-test-proof-current-20260811T205844Z-clean-current-seeded-fresh-isolated-plan

| ID | Persona / precondition | Browser actions | Interaction evidence | Expected observable result | Status | Evidence | Related work units | Run cache |
|---|---|---|---|---|---|---|---|---|
| US-01 | Keyboard/mouse user opening the local HTML file | Open button-chain.html, click the current last visible button five times from the initial state | Mouse click on the visible bottom-most/current last button for each chain step | Clicks 1-4 each append exactly one lower button; click 5 activates generated button 4 and leaves only lowercase finished with a visible white border | 💤 untested | Planning-only proof; browser execution is deferred to W06. | W01,W02,W03,W04,W05,W06,W07 | `ui-story-runs/US-01.md` |
