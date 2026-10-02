# tmux

### Identity
Terminal multiplexer: multiple shell sessions (panes/windows) inside one
terminal, persistent server outlives any one attached client. `tmux`
(attach/create default session) or `tmux new-session -s NAME`.

### Layout
- Body -> whatever runs in the focused pane (a shell prompt by default), no
  fixed content layout otherwise
- Last row -> status bar, default format `[session-name]
  window-index:window-name*  "hostname" HH:MM DD-Mon-YY` (configurable via
  `.tmux.conf`, not a guarantee)
- Split pane draws a `|` (vertical) or horizontal rule between panes

### Modes
Keys go to the focused pane's program UNLESS the prefix key (CTRL-B
default) was just pressed, putting tmux into a one-keystroke command state:
the next key is a tmux command, after which pass-through resumes. No
persistent "tmux mode" -- always prefix, then exactly one command key.

### Keys
All below are prefixed with CTRL-B (send as two wrapper actions, `key
CTRL-B` then the command key, not a single combo).

| Key (after CTRL-B) | Action | Notes |
|-----|--------|-------|
| % | Split pane vertically | Creates a pane to the right, `\|` divider visible |
| " | Split pane horizontally | |
| arrow key | Move focus to adjacent pane | |
| z | Zoom/maximize focused pane | [unconfirmed] |
| { / } | Resize/swap/rotate panes | [unconfirmed] |
| d | Detach | Attached client exits; session/panes keep running on the server. Reattach: `tmux attach -t NAME` in a new wrapper session. |
| c | Create new window | |
| n / p | Next/previous window | |
| w | Window list/selection | [unconfirmed] |
| x | Kill focused pane | Prompts to confirm |
| [ | Enter copy-mode (scroll history) | [unconfirmed] |

### Workflows
1. Run a second command alongside: CTRL-B then `%` (or `"`), check a
   divider appeared, drive the new (now-focused) pane.
2. Detach: CTRL-B then `d`. Note the session name from the status bar to
   reattach later.

### Quirks
- Detaching ends the wrapper's own session when tmux was started as its
  direct child (the client process exiting is what the wrapper tracks) --
  the tmux session itself survives; reattach with a new wrapper session.
- Shell startup noise (rc-file warnings etc.) inside a pane is
  environment-specific, not a tmux behavior.

### Unconfirmed
- Copy-mode (`[`): reachable/usable through this wrapper for scrolling
- Pane resize (`{`/`}`): exact behavior, whether incremental
- Window list (`w`): interactive picker vs. cycle-to-next
- Pane zoom (`z`): toggle behavior
