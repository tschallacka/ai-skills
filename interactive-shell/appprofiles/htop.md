# htop

### Identity
Interactive color process monitor: per-core meters, tree view, mouse
support, on-screen function-key menu. `htop`. Refreshes on its own timer.

### Layout
- Top rows -> per-core CPU meters (`[||||...]` bar + percentage per core or
  pair), task/load-average/uptime summary lines, `Mem[...]`/`Swp[...]`
  usage bars
- Below that -> blank separator, then tab labels (`[Main] [I/O]`) if
  multiple screens configured
- Process table -> own header row (`PID USER PRI NI VIRT RES SHR S CPU%
  MEM% TIME+ Command` by default)
- Last row -> function-key bar (`F1Help F2Setup F3Search F4Filter F5Tree
  F6SortBy F7Nice- F8Nice+ F9Kill F10Quit`)

### Colors
- meters and some process rows report highlighted/color-tagged (`fg-cyan`
  for load/meter text, `fg-magenta` for some usernames, `bold` for some
  numeric fields) -- htop's own color scheme, NOT a selection indicator.
  Use the actual movable selection cursor (full highlighted row) for
  selection.

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| F3 | Search | |
| F4 | Filter | Opens `Filter:` prompt; typed text filters the table live, each keystroke narrows further. ESC clears filter, returns to full list. |
| F5 | Tree view | Toggles parent/child indented tree |
| F6 | Sort by | Opens a column picker |
| F9 | Kill | Prompts for a signal, applies to selected row |
| F10 / q | Quit | No confirmation |
| Arrow keys | Move selection cursor | |
| SPACE | Tag/untag selected process | For multi-process actions |

### Workflows
1. Find a process: F4, type a distinguishing substring, check table
   narrowed to matching rows, ESC to clear filter when done.
2. Quit: F10 (or `q`).

### Quirks
- Process table columns are plain whitespace-separated (no embedded pipes,
  contrast mc's pipe-delimited panel columns); `markup` table synthesis
  handles htop cleanly.

### Unconfirmed
- Whether F4 filter is case-sensitive
- Whether filter matches full paths or only the displayed command name
