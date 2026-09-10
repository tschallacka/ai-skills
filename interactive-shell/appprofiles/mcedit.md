# mcedit (Midnight Commander's built-in editor)

### Identity
mc's own modeless text editor. `mcedit FILE`. Reached from mc's F4 only when
mc is configured to use its internal editor rather than `$EDITOR` (mc.md).

### Layout
- Row 1 -> status line: filename (possibly `~`-abbreviated), modified flag
  (`[----]` clean, `[-M--]` modified), cursor line/column, byte
  offset/file size
- Body -> buffer, one line per row
- Last row -> function-key bar (`1Help 2Save 3Mark 4Replac 5Copy 6Move
  7Search 8Delete 9PullDn10Quit`)

### Modes
Not modal -- typed characters always insert. Dialogs are a separate
interaction surface: while one is open, keys go to the dialog, not the
buffer.

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| F2 | Save | Opens the "Confirm save file" dialog; does NOT write by itself |
| ENTER (in dialog, after F2) | Confirm save | `[ Save ]` is the default button |
| F10 | Quit | Prompts to save first if modified |
| F3 | Mark (start/extend selection) | |
| F5 | Copy marked block | |
| F6 | Move marked block | |
| F7 | Search | |
| F8 | Delete marked block | |

### Dialogs
- **Save confirmation** (F2): title "Save file", message `Confirm save
  file: "<path>"`, buttons `[ Save ]` `[ Cancel ]`. ENTER activates the
  default (`[ Save ]`). Button navigation beyond ENTER-on-default
  [unconfirmed].

### Workflows
1. Edit and save: type the change, F2, wait for "Confirm save file" dialog,
   ENTER, check on-disk content changed. F10 to quit (no further prompt when
   clean).
2. Discard and quit: F10 with a modified buffer prompts to save; decline to
   exit without writing.

### Quirks
- F2 alone does not save -- it opens a confirmation dialog that itself must
  be confirmed (ENTER). Checking only the buffer rows, not the full screen,
  misses this dialog.

### Unconfirmed
- Whether the save-confirmation dialog is configurable (on/off) in mc's Options
- Button navigation inside the save dialog beyond ENTER-on-default (TAB/arrows not tested)
