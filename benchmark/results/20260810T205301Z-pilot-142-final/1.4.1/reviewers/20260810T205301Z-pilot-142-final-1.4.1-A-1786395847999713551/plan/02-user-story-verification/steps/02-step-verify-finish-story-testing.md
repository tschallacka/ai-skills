# Testing companion: 02-step-verify-finish-story

## Browser verification

Run `ui-story-runs/US-02.md` after US-01 passes with five visible buttons. Click the fourth generated button through the rendered UI and record whether the previous buttons are gone, the text is exactly `finished`, and a visible white border is present.

## Pass criteria

US-02 may be marked passed only when the document is cleared and the only completion content is exact lowercase `finished` with a visible white border. Any failure must create or update `bugs.md` before retrying.
