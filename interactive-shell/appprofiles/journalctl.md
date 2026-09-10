# journalctl

### Identity
Systemd journal viewer and query tool. `journalctl` (interactive pager, shows recent entries), `journalctl -f` (follow mode, shows new entries live), `journalctl -n N` (show last N entries then exit), `journalctl --no-pager` (non-interactive, print to stdout).

### Layout (default pager mode)
- Status line at bottom (current line number, total, percent scrolled — typical less-style pager)
- Body -> journal entries, one per line, chronological order (oldest first by default)
- Less-style navigation (same as less.md)

### Layout (follow mode, -f)
- Live updating output, most recent at bottom
- No pager, no line numbering
- CTRL-C to stop

### Modes
- **Pager mode** (default): Shows journal in a less-like pager, full navigation available. Exit with `q`.
- **Follow mode** (`-f`): Displays new entries as they arrive, no paging/scrolling, just append to screen. CTRL-C to stop.
- **Non-interactive mode** (`--no-pager`): Prints entries to stdout and exits, no screen refresh.

### Keys (pager mode only)
Inherits less navigation (see less.md). Relevant commands:
| Key | Action | Notes |
|-----|--------|-------|
| SPACE | Page down | |
| b | Page up | |
| g | Go to beginning | |
| G | Go to end | |
| / | Search | Regex forward |
| q | Quit | |

### Workflows
1. View recent entries with paging: `journalctl`, navigate with SPACE/b/g/G, exit with `q`.
2. Follow new entries (tail-like): `journalctl -f`, watch entries appear, CTRL-C to stop.
3. Show last 10 entries and exit: `journalctl -n 10 --no-pager`.

### Quirks
- Default pager is less, inheriting all less behavior.
- Follow mode (`-f`) has NO paging — it is a stream, not a screen interface.
- Without `--no-pager`, journalctl enters a pager even for short output (if output is longer than screen height).

### Unconfirmed
- Whether journalctl respects PAGER environment variable
- Whether filtering options (`-u`, `--since`, `--until`) change the pager behavior
