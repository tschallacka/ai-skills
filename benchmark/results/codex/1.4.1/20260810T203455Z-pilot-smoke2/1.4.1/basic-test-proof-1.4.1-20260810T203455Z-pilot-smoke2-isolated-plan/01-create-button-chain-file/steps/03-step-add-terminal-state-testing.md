# Testing companion: 03-step-add-terminal-state

## Browser verification

This behavior is verified downstream by `W05` / `US-01`. After generated button four exists, click generated button four through the rendered UI.

Pass criteria: the document is cleared of all prior buttons and visible extra content, and the remaining completion state shows exact lowercase text `finished`. Fail criteria: terminal state triggers before generated button four is clicked, old buttons remain, extra visible text remains, or the final text differs in spelling or casing.

## Automated tests

No separate automated test file is planned. The direct browser story owns the terminal-state proof.
