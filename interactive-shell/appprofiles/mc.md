# mc (Midnight Commander)

### Identity
Two-pane orthodox file manager. `mc [dir1] [dir2]`. `mc -u DIR` runs with an
isolated config/state directory (`$MC_HOME` under it).

### Layout
- Row 1 -> menu bar (`Left File Command Options Right`), opened with F9
- Rows 2..(rows-5) -> two side-by-side panes, each with a title bar (its
  directory), a column header (`Name | Size | Modify time` by default), a
  free-space footer; one pane active (has the cursor), TAB switches
- Below panes -> one-line hint, then a `$ ` shell command line an agent can
  type shell commands into directly
- Last row -> function-key bar (`1Help 2Menu 3View 4Edit ...`)

### Colors
- `fg-white bg-blue` -> directory
- highlighted (reverse-video or color-outlier via `elements`/`markup`) ->
  cursor row in the active pane; this IS the selection, not decoration
- other file-type colors (executables, symlinks, archives) follow the
  active skin, not catalogued [unconfirmed]

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| TAB | Switch active pane | |
| ENTER | Open selection | Directory: cd into it. Executable file: runs it. Other: opens the configured viewer/editor for its extension. |
| F3 | View (read-only) | |
| F4 | Edit | Opens `$EDITOR`/`$VISUAL` if set, not mcedit, on the build checked here (B125) -- confirm per build before relying on it |
| F5 | Copy | Opens a dialog, see Dialogs |
| F6 | Move/rename | Opens a dialog, see Dialogs |
| F7 | Mkdir | Opens a dialog, see Dialogs |
| F8 | Delete | Opens a confirmation dialog, see Dialogs |
| F9 | Open menu bar | |
| F10 | Quit | |
| PAGEUP / PAGEDOWN | Scroll active pane's listing | Ordinary list scrolling, not overloaded (contrast B146) |
| ESC | Cancel current dialog | Closes without confirming |

Typing a visible filename does NOT search the active pane; it types into the
command line below the panes. Quick-search key binding not confirmed
[unconfirmed] -- use arrow/PAGEUP/PAGEDOWN instead.

### Dialogs
- **F7 Mkdir**: title "Create a new Directory", text field "Enter directory
  name:", buttons `[< OK >]` `[ Cancel ]`. Field/button TAB order
  [unconfirmed]. ENTER on OK confirms; ESC cancels. Confirmed: creates the
  directory in the active pane.
- **F8 Delete**: title "Delete", message "Delete '<name>'?", buttons
  `[ Yes ]` `[ No ]`. Button TAB/arrow order [unconfirmed]. ENTER on Yes
  confirms; ESC or No cancels. Confirmed: ESC cancels, nothing deleted.
- **F5 Copy / F6 Move**: prompts for a destination path or new name. Exact
  field layout and buttons [unconfirmed].

### Workflows
1. Edit and save via `$EDITOR`: set `EDITOR=nano` before starting mc.
   Navigate to file, F4. Check screen shows that editor's UI on the expected
   filename. Use that editor's own save keys (see nano.md).
2. Copy/move: select file, F5 or F6, confirm destination in the dialog,
   ENTER.

### Quirks
- F4 opened `$EDITOR` (nano), not mcedit, on this build -- contradicts an
  earlier finding (B125) that F4 always opens mcedit. Re-verify per
  build/config.
- Box-drawing borders reach the wrapper only as ASCII `-`/`|`/`+`; `markup`
  pane/table detection is scoped to that shape.

### Unconfirmed
- TAB/arrow navigation order inside mkdir/delete/copy/move dialogs
- F5/F6 destination-prompt exact layout and button names
- Quick-search key binding
