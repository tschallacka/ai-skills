# ed

### Identity
Minimal line editor, POSIX standard. `ed FILE`. Line-oriented, terse output (prints nothing unless told to). No prompt visible — just waits for commands.

### Layout
- No visible prompt or status bar
- Output appears only when requested (print commands, or when reporting file size/write count)
- Scrollback shows prior commands and responses

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| RETURN | Execute command | Required after typing a command |
| . (dot, alone on a line in input mode) | End input mode | Ends append/insert input, returns to ready state |
| ESC | Cancel current input mode | Returns to ready state without saving input |

### Workflows
1. Print a line: `1p` (print line 1), RETURN. For all lines: `%p` or `1,$p`, RETURN.
2. Print line count: `=` (shows current line or last line with just `=`), RETURN.
3. Append text: `$a` (after last line) or `3a` (after line 3), RETURN, type text, RETURN, `.` alone, RETURN.
4. Insert before line: `3i`, RETURN, type text, RETURN, `.` alone, RETURN.
5. Delete lines: `1d` (delete line 1), `1,5d` (delete lines 1-5), RETURN.
6. Save and quit: `w` (write, prints byte count), RETURN, `q` (quit), RETURN.

### Quirks
- ed prints the file size (in bytes) when first opening a file.
- ed prints the byte count (not line count) when writing with `w`.
- ed is utterly silent on successful append/insert/delete — no confirmation.
- No visible prompt; all feedback is via output from explicit print or status commands.
- Unlike ex, ed does not print command echoes; commands are interpreted silently.

### Unconfirmed
- Whether ed supports regex patterns in search
- Whether address ranges like `1,$` are supported vs. only numeric addresses
