# dialog

### Identity

ncurses dialog-box toolkit for creating modal dialogs, menus, and forms in shell scripts. `dialog --menu`, `dialog --yesno`, `dialog --inputbox`, `dialog --checklist`, etc.

### Options

| Flag | Effect | Notes |
|------|--------|-------|
| `--menu TEXT HEIGHT WIDTH MENU-HEIGHT TAG ITEM ...` | Show a menu of tagged items | Returns selected TAG to stdout; exit code 0 if selected, 1 if Cancel |
| `--yesno TEXT HEIGHT WIDTH` | Show Yes/No buttons | Exit code 0 for Yes, 1 for No |
| `--inputbox TEXT HEIGHT WIDTH [INIT]` | Show a single-line text input | Returns entered text to stdout |
| `--checklist TEXT HEIGHT WIDTH ITEM-HEIGHT TAG ITEM STATUS ...` | Show checkboxes | Returns selected TAGs to stdout, one per line |
| `--radiolist TEXT HEIGHT WIDTH ITEM-HEIGHT TAG ITEM STATUS ...` | Show radio buttons | Returns selected TAG to stdout |

### Layout

- Top -> boxed title with dialog title text
- Body -> list of menu items (tag and label pairs) or form fields, with current selection highlighted
- Bottom -> OK/Cancel (or Yes/No, or similar) buttons; function-key hints below

### Keys

| Key | Action | Notes |
|-----|--------|-------|
| UP / DOWN | Move between menu items or form fields | |
| LEFT / RIGHT | Move between buttons at the bottom | |
| TAB | Move focus between menu/list and the button area | |
| SPACE | Toggle checkbox state (in `--checklist` mode) | |
| ENTER | Activate focused button (OK, Yes, or equivalent) | |
| ESC | Activate Cancel button | |
| / | Search for text in menu (starts incremental search) | |

### Workflows

1. **Select from menu**: DOWN to highlight choice, ENTER to select.
2. **Check multiple items**: DOWN to move between items, SPACE to toggle checkbox, ENTER to confirm.
3. **Enter text**: Type in input field, ENTER to confirm.
4. **Cancel**: ESC or navigate to Cancel button and press ENTER.

### Quirks

- Exit code (not stdout) signals the user's choice: 0 for OK/Yes/selected, 1 for Cancel/No, 127 if ESC pressed without confirmation.
- Selected value goes to stdout; dialog's own messages and status go to stderr.
- `--separate-output` option in checklist mode lists selected items one per line instead of space-separated.

### Unconfirmed

- Exact behavior of `/` search key [unconfirmed]
- Multiple-selection dialogs' full range of options [unconfirmed]
