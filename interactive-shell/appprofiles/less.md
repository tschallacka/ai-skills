# less

### Identity
Read-only pager. `less FILE`, or reads stdin from a pipe.

### Layout
- Body -> file content, one line per row (long lines wrap or truncate
  depending on options)
- Last row -> status/prompt: filename, a `:` prompt, or in-progress search
  text, depending on the last action

### Modes
Not modal for reading (never modifies the file). `/` or `?` puts the status
line into search-text entry until ENTER (execute) or ESC (cancel).

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| SPACE / PAGEDOWN | Next page | |
| b / PAGEUP | Previous page | |
| j / DOWN | Down one line | |
| k / UP | Up one line | |
| g | Go to first line | |
| G | Go to last line | |
| `/text` ENTER | Search forward | Jumps to and displays the match at the top |
| `?text` ENTER | Search backward | |
| n / N | Repeat search forward/backward | |
| q | Quit | No confirmation |

### Workflows
1. Find a line: `/` + distinctive text, ENTER. Check the match is visible
   before reading context around it.
2. Read to end and quit: `G`, then `q`.

### Quirks
None found.

### Unconfirmed
- Status-line format when not freshly opened or mid-search
