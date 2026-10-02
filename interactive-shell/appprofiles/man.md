# man

### Identity
Manual page viewer, pager-based. `man COMMAND` to view the manual for a command. `man -k KEYWORD` searches for manual pages (one-shot, non-interactive).

### Layout
- Body -> manual page content, scrollable
- Last row -> status line in format "Manual page COMMAND(SECTION) line LINE (press h for help or q to quit)"

### Modes
Read-only pager mode (same as less.md); `/` or `?` for search mode.

### Keys
Same as less.md (man uses less as its pager by default). See less.md for complete key bindings.

| Key | Action | Notes |
|-----|--------|-------|
| SPACE / PAGEDOWN | Next page | |
| b / PAGEUP | Previous page | |
| j / DOWN | Down one line | |
| k / UP | Up one line | |
| g | Go to first line | |
| G | Go to last line | |
| `/text` ENTER | Search forward | |
| `?text` ENTER | Search backward | |
| n / N | Repeat search forward/backward | |
| q | Quit | |

### Workflows
1. View a manual page: `man ls`. Observe page content. Press SPACE to scroll down. Press q to quit.
2. Search within a manual page: `man ls`, press `/`, type "option", ENTER. Observe the search result highlighted. Press n to find next match. Press q to quit.
3. Keyword search (non-interactive): `man -k passwd`. Observe list of manual pages matching "passwd" keyword. No pager; output exits immediately.

### Quirks
- Status line format shows `Manual page COMMAND(SECTION)` instead of just filename
- `-k` flag performs a non-interactive keyword search; does not open a pager
- Man delegates to less (or another pager) for viewing; keybindings are from the pager, not man itself

### Unconfirmed
- Whether man uses less, more, or another pager on this system [unconfirmed]
- Behavior when a manual page does not exist
