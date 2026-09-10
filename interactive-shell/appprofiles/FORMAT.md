# App profile format

One Markdown file per application, read by an agent driving it through the
`interactive-shell` wrapper. Pure fact, no narrative. State what is true; tag
what was not directly confirmed with `[unconfirmed]` at the end of the line
rather than writing a sentence about how it was found. No "verified on
<date>" retrospective prose -- a fact is either stated plainly or marked
unconfirmed.

Installed to `${XDG_CONFIG_HOME:-~/.config}/tsch-ai-skills/appprofiles/`,
reinstalled (overwritten) on every install run. Not written to by an agent
at runtime -- a session-specific or machine-specific finding that
contradicts or extends a profile belongs in the agent's own memory
(`memory/tui-apps/<name>.md` per interactive-shell/SKILL.md), not here.

## Sections, in order. Omit one that does not apply.

### Identity
One line: what it is, the command(s) that start it.

### Options
Only if invocation-time flags materially change what an agent should
expect (a one-shot vs. repeating mode, a mode that changes the whole
interaction, e.g. `-f`/batch-file input). Table: `flag | effect | notes`.
Distinct from Keys, which covers keys pressed once the program is running.

### Layout
Bullet list, row/region -> content. What is FIXED regardless of what the
buffer/data currently holds.

### Colors
Bullet list, `attribute -> meaning`, only if the app assigns its own meaning
to color/highlight beyond generic emphasis. Omit if none.

### Modes
Only if modal. Bullet list per mode: name, on-screen tell, how to enter, how
to exit.

### Keys
Table: `key | action | notes`. Add a `mode` column if modal. Note explicitly
when a named key this wrapper sends does NOT do what its name suggests, and
what to send instead.

### Dialogs
One entry per dialog/prompt the app can show: trigger, its fields/buttons,
how focus moves between them (TAB, arrows, a shortcut letter), how to
confirm and how to cancel.

### Menus
Only if the app has a persistent menu bar or a full-screen settings/setup
view (distinct from a one-off Dialog): how to open it, how to navigate
between top-level items and into a submenu/section, how to change a
setting, how to close/save/cancel. One entry per menu/screen.

### Workflows
Numbered, key-presses-only sequences for common tasks. No narration between
steps beyond what must be verified on screen before the next key (name the
exact string/element to check for, not "confirm it worked").

### Quirks
Bullet list: a fact that would surprise someone extrapolating from a
generic TUI or the app's reputation.

### Unconfirmed
Bullet list: things worth knowing that were not checked.
