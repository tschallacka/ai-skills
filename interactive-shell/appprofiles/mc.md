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
| F4 | Edit | Opens whichever editor mc is configured for: its own built-in mcedit by default, or `$EDITOR`/`$VISUAL` when mc's `use_internal_edit` option is off. Check the next screen to see which one opened -- see mcedit.md or nano.md accordingly. |
| F5 | Copy | Opens a dialog, see Dialogs |
| F6 | Move/rename | Opens a dialog, see Dialogs |
| F7 | Mkdir | Opens a dialog, see Dialogs |
| F8 | Delete | Opens a confirmation dialog, see Dialogs |
| F9 | Open menu bar | |
| F10 | Quit | |
| PAGEUP / PAGEDOWN | Scroll active pane's listing | Ordinary list scrolling, not overloaded |
| ESC | Cancel current dialog | Closes without confirming |

Typing a visible filename does NOT search the active pane; it types into the
command line below the panes. Quick-search key binding not confirmed
[unconfirmed] -- use arrow/PAGEUP/PAGEDOWN instead.

### Menus
- **F9 Menu bar**: Top row shows `Left File Command Options Right`. F9 opens
  menu mode; DOWN opens the focused menu's dropdown; LEFT/RIGHT navigate
  between menu items; UP/DOWN scroll within a dropdown; ENTER/SPACE activates
  an item; ESC closes all menus.
  - **Left/Right panes**: File listing, Quick view (C-x q), Info (C-x i),
    Tree, Listing format..., Sort order..., Filter..., Encoding... (M-e),
    FTP/Shell/SFTP links, Panelize, Rescan (C-r).
  - **File**: View (F3), View file..., Filtered view (M-!), Edit (F4), Copy
    (F5), Chmod (C-x c), Link (C-x l), Symlink (C-x s), Relative symlink
    (C-x v), Edit symlink (C-x C-s), Chown (C-x o), Advanced chown, Chattr
    (C-x e), Rename/Move (F6), Mkdir (F7), Delete (F8), Quick cd (M-c).
  - **Command**: User menu (F2), Directory tree, Find file (M-?), Swap
    panels (C-u), Switch panels on/off (C-o), Compare directories (C-x d),
    Compare files (C-x C-d), External panelize (C-x !), Show directory sizes
    (C-Space), Command history (M-h), Viewed/edited files history (M-E),
    Directory hotlist (C-\), Active VFS list (C-x a), Background jobs (C-x j),
    Screen list (M-`), Undelete files, Edit extension/menu/highlighting files.
  - **Options**: Configuration..., Layout..., Panel options..., Confirmation...,
    Appearance..., Display bits..., Learn keys..., Virtual FS..., Save setup.

### Dialogs
- **F7 Mkdir**: title "Create a new Directory", text field "Enter directory
  name:", buttons `[< OK >]` `[ Cancel ]`. ENTER on default OK confirms; ESC
  cancels. Confirmed: creates the directory in the active pane.
- **F8 Delete**: title "Delete", message "Delete '<name>'?", buttons
  `[ Yes ]` `[ No ]`. ENTER on default button confirms; ESC or No cancels.
  Confirmed: ESC cancels, nothing deleted.
- **F5 Copy / F6 Move**: prompts for a destination path or new name. Exact
  field layout and buttons [unconfirmed].

### Workflows
1. Edit and save: navigate to file, F4. Check the next screen to identify
   which editor opened (mcedit.md or nano.md/vi.md/etc.), then use that
   editor's own save keys.
2. Copy/move: select file, F5 or F6, confirm destination in the dialog,
   ENTER.

### Quirks
- Box-drawing borders reach the wrapper only as ASCII `-`/`|`/`+`; `markup`
  pane/table detection is scoped to that shape.

### Unconfirmed
- TAB/arrow navigation order inside mkdir/delete/copy/move dialogs (ENTER confirms default)
- F5/F6 destination-prompt exact field layout and button names
- Quick-search key binding in file panes
