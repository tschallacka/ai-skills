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
| F2 | Setup | Opens configuration screen (see Menus) |
| F3 | Search | |
| F4 | Filter | Opens `Filter:` prompt; typed text filters the table live, each keystroke narrows further. ESC clears filter, returns to full list. |
| F5 | Tree view | Toggles parent/child indented tree |
| F6 | Sort by | Opens a column picker |
| F9 | Kill | Prompts for a signal, applies to selected row |
| F10 / q | Quit | No confirmation |
| Arrow keys | Move selection cursor | |
| SPACE | Tag/untag selected process | For multi-process actions |

### Menus
- **F2 Setup**: Multi-section configuration screen accessed via F2. Categories
  on the left (Display options, Header layout, Meters, Screens, Colors);
  DOWN/UP navigate between categories; content changes on the right side per
  category. F10 exits Setup and saves changes.
  - **Display options**: Checkboxes for Tree view, Shadow other users, Hide
    kernel/userland threads, Hide container processes, Thread colors, Custom
    thread names, Program path, Basename highlight, Outdated program
    highlighting, etc.
  - **Header layout**: Column layout options (1 column full-width, 2 columns
    various splits 50/50/33/67/etc., 3 columns various splits).
  - **Meters**: Left/right column selections for meters (CPUs, Memory, Swap,
    Task counter, Load average, Uptime) with available meter list (Clock, Date,
    Date and Time, Load averages, Memory, Swap, Combined memory/swap, System,
    HugePages, Task counter, Uptime, Battery, Hostname).
  - **Screens**: Main and I/O screen tabs; active/available column pickers for
    customizing which process information is displayed.
  - **Colors**: Color scheme selector (Default, Monochromatic, Black on White,
    Light Terminal, MC, Black Night, Broken Gray).

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
