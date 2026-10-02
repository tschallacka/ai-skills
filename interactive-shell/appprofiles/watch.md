# watch

### Identity
Runs a command repeatedly, redrawing its output on a fixed interval. `watch
COMMAND` or `watch -n SECONDS COMMAND` (default interval 2s).

### Layout
- Row 1 -> `Every N.Ns: COMMAND    hostname: timestamp`
- Rows 2+ -> COMMAND's own output, redrawn each interval, no input needed

### Modes
Not modal -- output redraws automatically; no dialogs from watch itself
(the wrapped command's own prompts, if any, behave normally).

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| q | Quit | Does NOT quit |
| CTRL-C | Quit | Confirmed: exits cleanly |
| SPACE | Force immediate refresh | [unconfirmed] |
| d | Highlight differences between updates | [unconfirmed] |

### Workflows
1. Monitor live: start watch, read output, wait for the timer to redraw.
2. Exit: CTRL-C.

### Quirks
- `q` does not quit, unlike most other read-only TUI programs (top, htop,
  less). Use CTRL-C.

### Unconfirmed
- Confirmation of SPACE forcing immediate refresh (sent as text, refreshed at next interval)
- Whether `d` toggles diff highlighting (visual confirmation difficult via terminal)
- Any in-session help/keybinding list (watch has minimal interactive keys)
