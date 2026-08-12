# Testing companion: 01-step-verify-append-story

## Browser verification

Run `ui-story-runs/US-01.md` after implementation. Use only normal browser input: load the future local file, click the current last button four times, and record the observable button count and placement after each click.

## Pass criteria

US-01 may be marked passed only when exactly one new button appears per click, below the clicked last button, five buttons are visible after four append clicks, and no `finished` completion state appears before the fourth generated button is clicked.
