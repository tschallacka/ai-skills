# alsamixer

Not tested this session -- no ALSA sound device exists in this environment
(`alsamixer` fails immediately: `cannot open mixer: No such file or
directory`). Every fact below is standard, long-stable documented
behavior, not independently confirmed here. [unconfirmed] throughout.

### Identity
ALSA volume-mixer TUI. `alsamixer`, or `alsamixer -c N` for a specific
sound card.

### Layout
- Top -> card/chip name, current view (Playback/Capture/All)
- Body -> one vertical volume-bar column per channel/control, each with a
  live percentage and an `MM`/`00` mute indicator below the bar
- Bottom -> function-key hints

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| LEFT / RIGHT | Move between channel columns | |
| UP / DOWN | Raise/lower the selected channel's volume | |
| m | Toggle mute on the selected channel | |
| SPACE | Toggle capture on/off (in Capture/All view) | |
| TAB | Switch view: Playback / Capture / All | |
| F6 | Select sound card | Opens a card-selection list |
| F1 / h | Help | |
| ESC | Quit | |

### Workflows
1. Adjust a channel: LEFT/RIGHT to select it, UP/DOWN to change volume.
2. Mute/unmute: select channel, `m`.
3. Quit: ESC.

### Unconfirmed
- Everything above -- no ALSA device available to test against in this
  environment
