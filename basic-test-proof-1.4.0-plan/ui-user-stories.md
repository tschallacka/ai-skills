# UI user stories: basic-test-proof-1.4.0-plan

| ID | Persona / precondition | Browser actions | Interaction evidence | Expected observable result | Status | Evidence | Related work units | Run cache |
|---|---|---|---|---|---|---|---|---|
| US-01 | A user opens the future button-chain.html file in a rendered browser with the initial button visible | Click the initial button, then click each newly appended current last button in order until the fourth generated button is activated | Mouse click on the visible initial button and each visible newly appended current last button | The fourth generated-button activation clears the document and shows finished with a visible white border; no extra button-chain controls remain | ⏭️ excluded | User-approved exclusion: the proof explicitly forbids creating, opening, serving, or testing HTML/browser artifacts; future execution must replace this with browser evidence. | W03 | `ui-story-runs/US-01.md` |
