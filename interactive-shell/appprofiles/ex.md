# ex

### Identity
POSIX line editor, vi's command mode. `ex FILE` or `vi -e FILE`. Line-oriented text manipulation — each command typed at the `:` prompt, ENTER to execute, output prints above the prompt. No visual screen editing.

### Layout
- Top -> scrollback of prior commands and their output
- Current line -> the `:` prompt, waiting for a command

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| RETURN | Execute command | Required after typing a command at the prompt |
| . (dot, alone on a line in input mode) | End input mode | Ends append/insert/change input, returns to `:` prompt |
| ESC | Cancel current input mode | Returns to `:` prompt without saving input |

### Dialogs
**Input mode (append, insert, change)**: Triggered by commands like `a`, `i`, `c`. No prompt visible — the screen will show the prior context but no `:` prompt. Type one or more lines of text. End input by typing `.` on its own line, then RETURN. ESC also returns to `:` prompt.

### Workflows
1. Print lines: `1,$p` (print all), `1p` (print line 1), `2,5p` (print lines 2-5) — command, RETURN.
2. Append text after a line: address + `a` (e.g., `3a`), RETURN, type text, RETURN, `.` alone on a line, RETURN.
3. Save and quit: `w` (write), RETURN, `q` (quit), RETURN.
4. Save to a different file: `w newfile` (e.g., `w /tmp/out.txt`), RETURN.

### Quirks
- Addresses can be line numbers (1, 2, 5), ranges (1,5 or 1,$), or regex patterns (/pattern/ or ?pattern?).
- The dot `.` at the end of input is a literal dot that terminates input mode, distinct from ex's use of `.` as "current line" in commands.
- ex is silent on success (no "OK" message for append/write); status is reported only for certain commands like `w`.

### Unconfirmed
- Whether ex supports regexes in the search commands (`/pattern/`, `?pattern?`)
- Whether tab completion exists
