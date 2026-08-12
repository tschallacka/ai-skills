# Step: 02-step-verify-finish-story

## Ownership

- Goal: `02-user-story-verification`
- Work unit: `W06`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `Browser flow US-02 fourth-generated finish`
- Subscope: `N/A`

## Objective

§ 4.1
Verify by direct browser clicks that pressing the fourth generated button clears the document and shows exactly finished with a visible white border.

## Instructions

§ 5.1
After US-01 passes, execute ui-story-runs/US-02.md exactly by clicking the fourth generated button in the rendered UI. Record browser evidence in ui-user-stories.md and the run cache.

## Acceptance criteria

§ 6.1
US-02 passes only if the click clears the previous buttons and renders only the exact lowercase text finished with a visibly white border. Any mismatch creates a bugs.md entry and follow-up investigation and fix goals.

## Handoff

§ 7.1
When W05 and W06 pass with no open bugs, the future implementation satisfies the requested UI behavior.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
