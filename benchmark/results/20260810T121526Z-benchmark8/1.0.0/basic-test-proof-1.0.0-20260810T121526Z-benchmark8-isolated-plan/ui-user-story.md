# UI user story: button-chain

As a user, I want each current-last-button click to reveal exactly one button
below it so that the chain grows predictably, and I want the fourth generated
button to end the interaction with a clear completion message.

## Acceptance scenarios

| Scenario | Action | Expected result |
|---|---|---|
| Initial | Load file | Exactly 1 button |
| Generate 1 | Click last button | Exactly 2 total buttons |
| Generate 2 | Click new last button | Exactly 3 total buttons |
| Generate 3 | Click new last button | Exactly 4 total buttons |
| Generate 4 / finish | Click new last button | Document has 0 buttons and exact `finished` with visible white border |
| Guard | Click a non-last button before finish | No append and no count change |

The story deliberately distinguishes total buttons from generated buttons:
the terminal action is generated button 4, which is total button 5.
