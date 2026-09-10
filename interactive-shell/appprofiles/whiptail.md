# whiptail

Simpler ncurses dialog-box toolkit (from the newt library), similar to dialog but with a more modern flat-UI style. `whiptail --menu`, `whiptail --yesno`, `whiptail --inputbox`, `whiptail --checklist`, etc.

### Options

| Flag | Effect | Notes |
|------|--------|-------|
| `--menu TEXT HEIGHT WIDTH MENU-HEIGHT TAG ITEM ...` | Show a menu of tagged items | Returns selected TAG to stdout; exit code 0 if selected, 1 if Cancel |
| `--yesno TEXT HEIGHT WIDTH` | Show Yes/No buttons | Exit code 0 for Yes, 1 for No |
| `--inputbox TEXT HEIGHT WIDTH [INIT]` | Show a single-line text input | Returns entered text to stdout |
| `--checklist TEXT HEIGHT WIDTH ITEM-HEIGHT TAG ITEM STATUS ...` | Show checkboxes | Returns selected TAGs to stdout, space-separated or one per line with `--separate-output` |
| `--radiolist TEXT HEIGHT WIDTH ITEM-HEIGHT TAG ITEM STATUS ...` | Show radio buttons | Returns selected TAG to stdout |

### Layout

- Top -> boxed dialog with title and descriptive text
- Body -> list of menu items or form fields
- Bottom -> buttons (OK/Cancel, Yes/No, etc.), with focus indicated by highlighted button

### Keys

| Key | Action | Notes |
|-----|--------|-------|
| UP / DOWN | Move between menu items or form fields | |
| TAB | Move between buttons or between list and buttons | |
| SPACE | Toggle checkbox (in `--checklist` mode) | |
| ENTER | Activate focused button | |
| ESC | Activate Cancel/No button | |
| LEFT / RIGHT | Move between buttons | |

### Workflows

1. **Select from menu**: DOWN to move between items, ENTER to select.
2. **Check items**: DOWN to navigate, SPACE to toggle checkboxes, ENTER to confirm.
3. **Yes/No prompt**: TAB to move between Yes and No, ENTER to select.
4. **Text input**: Type text, ENTER to confirm.

### Quirks

- Exit code signals result: 0 for OK/Yes, 1 for Cancel/No.
- Output goes to stdout (unlike dialog's stderr).
- UI is slightly flatter and more "modern" than dialog; visually less boxed.
- Behaves very similarly to dialog but with fewer command-line options.

### Unconfirmed

- Full option compatibility with dialog [unconfirmed]
- Behavior of all lesser-used flags [unconfirmed]
