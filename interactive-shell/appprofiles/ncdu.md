# ncdu (NCurses Disk Usage)

### Identity
Interactive disk-usage analyzer, browsable like a file manager sorted by
size. `ncdu [DIR]` (defaults to cwd).

### Layout
- Row 1 -> `ncdu VERSION ~ Use the arrow keys to navigate, press ? for
  help`
- Row 2 -> current path
- Body -> one entry per row: `<flag> <size> [<bar>] <name>`, `<flag>` one
  of ` `/`e`/`!`/`.`/`<`/`>`/`@`/`^` (see Colors), `<bar>` a `#`-filled
  proportional size bar, directories shown as `/name`
- Last row -> `Total disk usage: X   Apparent size: Y   Items: N`

### Colors
- flag column, one character before size:
  - (blank) -> ordinary file/directory
  - `e` -> empty directory
  - `!` -> error reading this directory
  - `.` -> error reading a subdirectory
  - `<` -> excluded from statistics
  - `>` -> on another filesystem
  - `@` -> not a file or directory (symlink, socket, etc.)
  - `^` -> excluded Linux pseudo-filesystem

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| UP/DOWN, k/j | Move cursor | |
| RIGHT/ENTER | Open selected directory | |
| LEFT, `<`, h | Open parent directory | |
| n | Sort by name (toggles ascending/descending) | |
| s | Sort by size (toggles ascending/descending) | |
| C | Sort by item count (toggles ascending/descending) | |
| M | Sort by mtime (needs `-e` flag at launch) | |
| d | Delete selected file/directory | Confirmation dialog, see Dialogs |
| t | Toggle dirs-before-files when sorting | |
| ? | Open help | |
| q | Quit (or close help/dialog if one is open) | |

### Dialogs
- **Help** (`?`): tabbed overlay, tabs `1:Keys` `2:Format` `3:About`.
  Switch tabs by pressing the tab's digit key (`1`, `2`, `3`) directly, not
  TAB/arrows. `q` closes it.
- **Delete confirmation** (`d`): [unconfirmed] -- not tested this session.

### Workflows
1. Find what's using space: sort by size (`s`, default), open the largest
   entries (ENTER) to descend, `<`/LEFT/h to go back up.
2. Read the flag column before assuming an entry is a plain file/dir --
   `@`/`>`/`^` mean something else (see Colors).

### Quirks
- The 1-character flag column immediately before the size can look like
  visual noise/misalignment in a narrow view if not expected -- it is a
  real, meaningful column (see Colors), not a rendering artifact.

### Unconfirmed
- Delete-confirmation dialog's exact fields/buttons and cancel path
- Whether ncdu can write/refresh mid-session (e.g. a rescan key)
