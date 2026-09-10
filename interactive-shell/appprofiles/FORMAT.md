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
contradicts or extends a profile, or a profile for an app with none shipped,
belongs in the sibling `appprofiles.d/` directory (per
interactive-shell/SKILL.md), not here.

A file in `appprofiles.d/` follows this same format, but its first line must
be the literal marker `<!-- tui-app-profile: v1 -->` -- nothing that reads
that directory (including the tui-hint plugin) treats an unmarked file there
as a profile, since the directory is otherwise just wherever an agent
happens to drop a `.md` file. A vendor file under `appprofiles/` carries no
marker; its presence in that directory is enough.

## Sections, in order. Omit one that does not apply.

### Identity
One line: what it is, the command(s) that start it.

### Invocation
Only if the profile's filename does not, by itself, match the leading
program word a hook sees on a command line -- a git subcommand
(`git bisect` needs `git-bisect.md`), or a flag-gated mode. One extended
regex (`grep -E` syntax) per line, matched against the command line with any
leading `sudo`/`env`/`VAR=value` wrapper already stripped; the first line
that matches wins. Omit the section entirely when the filename's own name
(before `.md`) is already the leading word -- that is a hook's default
match and needs no declaration here.

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
