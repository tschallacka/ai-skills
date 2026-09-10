# screen (GNU Screen)

### Identity
Terminal multiplexer with a persistent server that outlives any one
attached client -- useful for a long-running process whose output must be
read back later, even across a wrapper session ending. `screen` or `screen
-S NAME` (named session, recommended so it can be reattached by name).

### Layout
- Body -> whatever runs in the current window (a shell prompt by default),
  no fixed content layout otherwise
- Last row -> status/message line, blank in the default config unless a
  message or copy-mode status is showing (e.g. `Copy mode - Column C Line
  L(+N) (cols,rows)` while in copy mode); no persistent status bar unless
  configured in `.screenrc`

### Modes
Keys go to the current window's program UNLESS the prefix key (CTRL-A by
default) was just pressed, putting screen into a one-keystroke command
state: the next key is a screen command, after which pass-through resumes.
**Copy mode** (entered with CTRL-A `[`) is a distinct, longer-lived mode for
scrolling and reading back output/scrollback: it stays active (arrow
keys/PAGEUP/PAGEDOWN move the view) until exited with ESC or ENTER
(ENTER also copies a mark), not just one keystroke.

### Keys
"After CTRL-A" keys are prefixed by the prefix key (send as two wrapper
actions: `combo a ctrl` then the command key, not a single combo). Copy
mode keys are sent directly, no prefix, once copy mode is entered.

| Key | Action | Mode | Notes |
|-----|--------|------|-------|
| c | Create a new window | After CTRL-A | |
| n / p | Next/previous window | After CTRL-A | |
| " | Window list (interactive picker) | After CTRL-A | [unconfirmed] |
| A | Rename current window | After CTRL-A | [unconfirmed] |
| S | Split region horizontally | After CTRL-A | [unconfirmed] |
| \| | Split region vertically | After CTRL-A | [unconfirmed] |
| Tab | Move focus to next region (after a split) | After CTRL-A | [unconfirmed] |
| [ | Enter copy mode | After CTRL-A | Enters a scrollable view, status line shows `Copy mode - ...` |
| d | Detach | After CTRL-A | Attached client exits (wrapper session ends if screen was its direct child); session/windows keep running server-side, listed as "Detached" by `screen -ls`. Reattach with `screen -r NAME` in a new wrapper session. |
| k | Kill current window | After CTRL-A | Prompts to confirm |
| \\ | Kill all windows and quit screen | After CTRL-A | Prompts to confirm |
| arrow keys, PAGEUP/PAGEDOWN | Move the scrollback view | Copy mode | |
| SPACE | Start a selection mark | Copy mode | [unconfirmed] |
| ENTER | End selection, copy it, exit copy mode | Copy mode | [unconfirmed] |
| ESC | Exit copy mode without copying | Copy mode | |

### Workflows
1. Start a long-running process that must survive a disconnect: `screen -S
   NAME`, run the command. CTRL-A then `d` to detach; the process keeps
   running. Reattach later from a fresh wrapper session with `screen -r
   NAME`.
2. Read back output that has scrolled off: CTRL-A then `[` to enter copy
   mode, PAGEUP/arrow keys to scroll, ESC to exit without altering
   anything.

### Quirks
- Detaching ends the wrapper's own session when screen was started as its
  direct child command (the client process exiting is what the wrapper
  tracks) -- the screen session itself survives; reattach with a new
  wrapper session pointed at `screen -r NAME`.
- No persistent status bar by default, unlike tmux -- the session name and
  window list are not visible on screen unless queried (CTRL-A `"`) or
  configured in `.screenrc`.

### Unconfirmed
- Window-list picker (`"`) layout and navigation
- Split-region commands (`S`, `\|`, region-switching with Tab)
- Copy-mode selection/copy mechanics (SPACE to mark, ENTER to copy) beyond
  scrolling
- Whether `screen -ls` and `-r NAME` are reachable/useful driven through
  the wrapper itself (tested only from a separate real shell this session)
